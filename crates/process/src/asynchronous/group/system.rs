use std::io;

#[cfg(target_os = "linux")]
pub(super) fn exited(id: u32) -> io::Result<bool> {
    let mut status = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            id,
            status.as_mut_ptr(),
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { status.assume_init().si_pid() } != 0)
}

#[cfg(target_os = "linux")]
pub(super) fn stop(id: u32) -> io::Result<()> {
    exited(id)?;
    let group = unsafe { libc::getpgid(id as libc::pid_t) };
    if group != id as libc::pid_t {
        return Err(io::Error::other("owned child lost its process group"));
    }
    let result = unsafe { libc::kill(-group, libc::SIGKILL) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn live(id: u32) -> io::Result<bool> {
    let mut count = 0;
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().parse::<u32>().is_err() {
            continue;
        }
        count += 1;
        if count > 65536 {
            return Err(io::Error::other("process observation exhausted"));
        }
        let text = match std::fs::read_to_string(entry.path().join("stat")) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        if member(&text, id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(target_os = "linux")]
fn member(text: &str, id: u32) -> io::Result<bool> {
    let (_, fields) = text
        .rsplit_once(") ")
        .ok_or_else(|| io::Error::other("malformed process observation"))?;
    let mut fields = fields.split_whitespace();
    let state = fields.next();
    let group = fields
        .nth(1)
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| io::Error::other("missing process group"))?;
    Ok(group == id && !matches!(state, Some("Z" | "X")))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn exited(_: u32) -> io::Result<bool> {
    Err(io::Error::other("process groups require Linux"))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn stop(_: u32) -> io::Result<()> {
    Err(io::Error::other("process groups require Linux"))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn live(_: u32) -> io::Result<bool> {
    Err(io::Error::other("process groups require Linux"))
}
