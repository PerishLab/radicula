use crate::error::Error;
use crate::udp::Mark;
use std::io;
use std::net::{SocketAddr, UdpSocket};

pub const IP: i32 = 0;
pub const TOS: i32 = 1;
pub const RECV: i32 = 13;
pub const UDP: i32 = 17;
pub const SEGMENT: i32 = 103;
pub const ENOPROTOOPT: i32 = 92;
pub const EOPNOTSUPP: i32 = 95;

pub fn denied(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(ENOPROTOOPT | EOPNOTSUPP))
}

#[cfg(target_os = "linux")]
mod ffi {
    use super::{IP, Mark, TOS};
    use std::io::{self, ErrorKind};
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    #[repr(C)]
    pub struct Iov {
        pub base: *mut u8,
        pub len: usize,
    }

    #[repr(C)]
    pub struct Msg {
        pub name: *mut u8,
        pub namelen: u32,
        pub iov: *mut Iov,
        pub iovlen: usize,
        pub control: *mut u8,
        pub controllen: usize,
        pub flags: i32,
    }

    #[repr(C)]
    pub struct Hdr {
        pub len: usize,
        pub level: i32,
        pub kind: i32,
    }

    unsafe extern "C" {
        pub fn setsockopt(fd: i32, ...) -> i32;
        pub fn getsockopt(fd: i32, ...) -> i32;
        pub fn recvmsg(fd: i32, msg: *mut Msg, flags: i32) -> isize;
    }

    fn align(len: usize) -> usize {
        let unit = size_of::<usize>();
        (len + unit - 1) & !(unit - 1)
    }

    fn data(hdr: *mut Hdr) -> *mut u8 {
        unsafe { (hdr as *mut u8).add(align(size_of::<Hdr>())) }
    }

    pub fn set(fd: i32, level: i32, name: i32, value: i32) -> Result<(), crate::error::Error> {
        let ret = unsafe { setsockopt(fd, level, name, (&value as *const i32).cast::<u8>(), 4u32) };
        if ret == 0 {
            Ok(())
        } else {
            Err(crate::error::Error::Io(io::Error::last_os_error()))
        }
    }

    pub fn get(fd: i32, level: i32, name: i32) -> Result<i32, crate::error::Error> {
        let mut value = 0i32;
        let mut len = 4u32;
        let ret = unsafe {
            getsockopt(
                fd,
                level,
                name,
                (&mut value as *mut i32).cast::<u8>(),
                &mut len,
            )
        };
        if ret == 0 {
            Ok(value)
        } else {
            Err(crate::error::Error::Io(io::Error::last_os_error()))
        }
    }

    pub fn take(fd: i32, buf: &mut [u8]) -> io::Result<(usize, SocketAddr, Mark)> {
        let mut name = [0u8; 128];
        let mut control = [0u8; 128];
        let mut iov = Iov {
            base: buf.as_mut_ptr(),
            len: buf.len(),
        };
        let mut msg = Msg {
            name: name.as_mut_ptr(),
            namelen: name.len() as u32,
            iov: &mut iov,
            iovlen: 1,
            control: control.as_mut_ptr(),
            controllen: control.len(),
            flags: 0,
        };
        let n = unsafe { recvmsg(fd, &mut msg, 0) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        let from = host(&name, msg.namelen)?;
        let mark = extract(&mut control, msg.controllen);
        Ok((n as usize, from, mark))
    }

    fn extract(control: &mut [u8], len: usize) -> Mark {
        if len < size_of::<Hdr>() {
            return Mark::Plain;
        }
        let hdr = unsafe { &*(control.as_ptr() as *const Hdr) };
        if hdr.len < size_of::<Hdr>() {
            return Mark::Plain;
        }
        if hdr.level == IP && hdr.kind == TOS {
            let tos = unsafe { *data(control.as_mut_ptr() as *mut Hdr) };
            return Mark::parse(tos);
        }
        Mark::Plain
    }

    fn host(buf: &[u8], len: u32) -> io::Result<SocketAddr> {
        let len = len as usize;
        if len < 8 {
            return Err(io::Error::from(ErrorKind::InvalidData));
        }
        let family = u16::from_ne_bytes([buf[0], buf[1]]);
        match family {
            2 => {
                let port = u16::from_be_bytes([buf[2], buf[3]]);
                let ip = Ipv4Addr::new(buf[4], buf[5], buf[6], buf[7]);
                Ok(SocketAddr::from((ip, port)))
            }
            10 if len >= 28 => {
                let port = u16::from_be_bytes([buf[2], buf[3]]);
                let mut oct = [0u8; 16];
                oct.copy_from_slice(&buf[8..24]);
                Ok(SocketAddr::from((Ipv6Addr::from(oct), port)))
            }
            _ => Err(io::Error::from(ErrorKind::InvalidData)),
        }
    }
}

pub fn set(socket: &UdpSocket, level: i32, name: i32, value: i32) -> Result<(), Error> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        ffi::set(socket.as_raw_fd(), level, name, value)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = socket;
        let _ = (level, name, value);
        Err(Error::Unsupported)
    }
}

pub fn get(socket: &UdpSocket, level: i32, name: i32) -> Result<i32, Error> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        ffi::get(socket.as_raw_fd(), level, name)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (socket, level, name);
        Err(Error::Unsupported)
    }
}

pub fn take(socket: &UdpSocket, buf: &mut [u8]) -> io::Result<(usize, SocketAddr, Mark)> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        ffi::take(socket.as_raw_fd(), buf)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (socket, buf);
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}
