mod worker;

use radicula_lock::Mode;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Seat(pub PathBuf);

impl Seat {
    pub fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "radicula-lock-{}-{stamp}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }

    pub fn file(&self) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.0.join("lease"))
            .unwrap()
    }

    pub fn spawn(&self, mode: Mode, backend: &str, action: &str) -> Peer {
        let marker = self
            .0
            .join(format!("ready-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "worker", "--nocapture"])
            .env("RADICULA_TEST_PATH", self.0.join("lease"))
            .env("RADICULA_TEST_MARKER", &marker)
            .env(
                "RADICULA_TEST_MODE",
                if mode == Mode::Shared {
                    "shared"
                } else {
                    "exclusive"
                },
            )
            .env("RADICULA_TEST_BACKEND", backend)
            .env("RADICULA_TEST_ACTION", action)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Peer { child, marker }
    }

    pub fn probe(&self, mode: Mode, backend: &str) -> i32 {
        self.spawn(mode, backend, "try").wait()
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

pub struct Peer {
    child: Child,
    marker: PathBuf,
}

impl Peer {
    pub fn wait(&mut self) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.code().expect("peer exited without a code");
            }
            assert!(Instant::now() < deadline, "peer timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.marker.exists() {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "peer exited before readiness"
            );
            assert!(Instant::now() < deadline, "peer readiness timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn pending(&mut self) {
        let deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < deadline {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "blocking peer returned while held"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn stop(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

pub fn worker() {
    worker::run();
}
