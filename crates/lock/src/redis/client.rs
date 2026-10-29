use super::{
    Config,
    key::Key,
    lease::{Lease, State},
    scripts,
    wire::{Reply, Wire},
};
use crate::{Error, Mode};
use std::{
    io,
    sync::{Arc, Condvar, Mutex, Weak},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub(super) struct Client {
    pub config: Config,
    pub epoch: String,
    leases: Mutex<Vec<Weak<Lease>>>,
    signal: Arc<(Mutex<bool>, Condvar)>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Client {
    pub fn connect(config: Config) -> Result<Arc<Self>, Error> {
        config.validate()?;
        let info = Wire::call(
            &config,
            &[
                b"INFO".to_vec(),
                b"server".to_vec(),
                b"replication".to_vec(),
                b"cluster".to_vec(),
                b"memory".to_vec(),
            ],
        )
        .map_err(|failure| Error::from(failure.error))?;
        let Reply::Text(info) = info else {
            return Err(protocol());
        };
        let info = std::str::from_utf8(&info).map_err(|_| protocol())?;
        let epoch = field(info, "run_id").ok_or_else(protocol)?;
        let major = field(info, "redis_version")
            .and_then(|value| value.split('.').next())
            .and_then(|value| value.parse::<u32>().ok());
        let supported = [
            major.is_some_and(|major| major >= 7),
            field(info, "role") == Some("master"),
            field(info, "cluster_enabled") == Some("0"),
            field(info, "maxmemory_policy") == Some("noeviction"),
        ];
        if supported.contains(&false) {
            return Err(Error::Unsupported(
                "Redis requires version7+, single primary, no cluster and noeviction",
            ));
        }
        if epoch.len() != 40 || !epoch.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(protocol());
        }
        let client = Arc::new(Self {
            config,
            epoch: epoch.into(),
            leases: Mutex::default(),
            signal: Arc::new((Mutex::new(false), Condvar::new())),
            worker: Mutex::default(),
        });
        Self::start(&client)?;
        Ok(client)
    }

    fn start(client: &Arc<Self>) -> Result<(), Error> {
        let weak = Arc::downgrade(client);
        let signal = client.signal.clone();
        let worker = thread::Builder::new()
            .name("radicula-redis".into())
            .spawn(move || {
                loop {
                    let stop = signal.0.lock().unwrap_or_else(|error| error.into_inner());
                    let (stop, _) = signal
                        .1
                        .wait_timeout(stop, Duration::from_millis(10))
                        .unwrap_or_else(|error| error.into_inner());
                    if *stop {
                        break;
                    }
                    drop(stop);
                    let Some(client) = weak.upgrade() else {
                        break;
                    };
                    let leases = {
                        let mut held = client
                            .leases
                            .lock()
                            .unwrap_or_else(|error| error.into_inner());
                        held.retain(|lease| lease.strong_count() > 0);
                        held.iter().filter_map(Weak::upgrade).collect::<Vec<_>>()
                    };
                    for lease in leases {
                        lease.tick(&client);
                    }
                }
            })
            .map_err(Error::from)?;
        *client
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(worker);
        Ok(())
    }

    pub fn acquire(&self, key: Arc<Key>, mode: Mode) -> Result<Option<Arc<Lease>>, Error> {
        key.admits()?;
        let mut leases = self
            .leases
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        leases.retain(|lease| lease.strong_count() > 0);
        if leases.len() >= self.config.slots {
            return Err(Error::Busy);
        }
        let started = Instant::now();
        let args = vec![
            self.epoch.as_bytes().to_vec(),
            key.policy().into_bytes(),
            match mode {
                Mode::Shared => b"shared".to_vec(),
                Mode::Exclusive => b"exclusive".to_vec(),
            },
            self.config.lease.as_millis().to_string().into_bytes(),
        ];
        let reply = self
            .eval(scripts::ACQUIRE, &key, &args)
            .map_err(|failure| {
                if failure.submitted {
                    key.fault("Redis admission is unconfirmed; no execution may start");
                    Error::Unconfirmed(Arc::new(failure.error))
                } else {
                    Error::from(failure.error)
                }
            })?;
        let Some(fence) = super::reply::admission(reply, &key, &self.epoch)? else {
            return Ok(None);
        };
        let epoch = &fence.epoch;
        let sequence = fence.sequence;
        let lease = Arc::new(Lease {
            key: key.clone(),
            token: format!("{epoch}:{sequence}"),
            fence,
            state: Mutex::new(State {
                end: started + self.config.lease,
                renew: started + self.config.renewal.unwrap_or(self.config.lease),
                lost: false,
                closing: false,
            }),
        });
        key.acquired();
        leases.push(Arc::downgrade(&lease));
        drop(leases);
        if lease.check().is_err() {
            let _ = lease.release(self);
            return Err(Error::Lost);
        }
        Ok(Some(lease))
    }

    pub fn renew(&self, lease: &Lease) -> Result<(), Error> {
        self.owned(scripts::RENEW, lease)
    }

    pub fn release(&self, lease: &Lease) -> Result<(), Error> {
        self.owned(scripts::RELEASE, lease)
    }

    fn owned(&self, script: &str, lease: &Lease) -> Result<(), Error> {
        let args = vec![
            self.epoch.as_bytes().to_vec(),
            lease.token.as_bytes().to_vec(),
            self.config.lease.as_millis().to_string().into_bytes(),
        ];
        let reply = self
            .eval(script, &lease.key, &args)
            .map_err(|failure| Error::Uncertain(Arc::new(failure.error)))?;
        let fields = array(reply)?;
        match code(&fields)? {
            1 => Ok(()),
            0 => Err(Error::Lost),
            -2 => Err(Error::Epoch),
            _ => Err(protocol()),
        }
    }

    fn eval(
        &self,
        script: &str,
        key: &Key,
        args: &[Vec<u8>],
    ) -> Result<Reply, super::wire::Failure> {
        let mut command = vec![
            b"EVAL".to_vec(),
            script.as_bytes().to_vec(),
            key.keys.len().to_string().into_bytes(),
        ];
        command.extend(key.keys.iter().cloned());
        command.extend(args.iter().cloned());
        Wire::call(&self.config, &command)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        *self
            .signal
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = true;
        self.signal.1.notify_all();
        if let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            && worker.thread().id() != thread::current().id()
        {
            let _ = worker.join();
        }
    }
}

fn field<'a>(info: &'a str, name: &str) -> Option<&'a str> {
    info.lines().find_map(|line| {
        line.strip_prefix(name)
            .and_then(|tail| tail.strip_prefix(':'))
    })
}

fn array(reply: Reply) -> Result<Vec<Reply>, Error> {
    match reply {
        Reply::Array(fields) => Ok(fields),
        _ => Err(protocol()),
    }
}

fn code(fields: &[Reply]) -> Result<i64, Error> {
    match fields.first() {
        Some(Reply::Integer(code)) => Ok(*code),
        _ => Err(protocol()),
    }
}

fn protocol() -> Error {
    Error::from(io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid Redis coordination reply",
    ))
}
