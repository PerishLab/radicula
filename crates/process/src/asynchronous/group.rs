mod system;

use std::io;
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::process::Child;

pub(super) fn configure(command: &mut Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = command;
}

pub(super) struct Group {
    id: u32,
    stopped: AtomicBool,
}

impl Group {
    pub(super) fn new(id: u32) -> Self {
        Self {
            id,
            stopped: AtomicBool::new(false),
        }
    }

    pub(super) async fn wait(&self, child: &mut Child) -> io::Result<ExitStatus> {
        while !system::exited(self.id)? {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.close(child, &mut None).await
    }

    pub(super) async fn close(
        &self,
        child: &mut Child,
        status: &mut Option<ExitStatus>,
    ) -> io::Result<ExitStatus> {
        if !self.stopped.load(Ordering::Acquire) {
            system::stop(self.id)?;
            self.stopped.store(true, Ordering::Release);
        }
        let observed = child.wait().await?;
        *status = Some(observed);
        let until = tokio::time::Instant::now() + Duration::from_secs(2);
        while system::live(self.id)? {
            if tokio::time::Instant::now() >= until {
                return Err(io::Error::other("process group did not drain"));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(observed)
    }
}
