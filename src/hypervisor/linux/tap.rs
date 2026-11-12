//! TAP device allocation for the dedicated per-microVM NIC.
//!
//! Opening `/dev/net/tun` and assigning an address requires `CAP_NET_ADMIN`.
//! The ioctls are the real ones; this environment typically returns `EPERM`,
//! which `Container::create` surfaces instead of pretending the NIC exists.

use std::fs::File;
use std::io::{self, Read, Write};
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use crate::error::{Error, Result};
use crate::resource::NetworkPlan;

const TUNSETIFF: libc::c_ulong = 0x4004_54CA;
const IFF_TAP: i16 = 0x0002;
const IFF_NO_PI: i16 = 0x1000;
const IFF_UP: i16 = 0x1;
const IFF_RUNNING: i16 = 0x40;
const SIOCSIFADDR: libc::c_ulong = 0x8916;
const SIOCSIFNETMASK: libc::c_ulong = 0x891C;
const SIOCSIFFLAGS: libc::c_ulong = 0x8914;

pub(crate) struct TapDevice {
    file: File,
    /// Interface name (`cn……`). Read by tests; production code closes the fd.
    #[allow(dead_code)]
    pub name: String,
}

impl TapDevice {
    pub(crate) fn open(plan: &NetworkPlan) -> Result<Self> {
        if plan.tap_name.len() > 15 {
            return Err(Error::Resource(format!(
                "tap name {} exceeds IFNAMSIZ",
                plan.tap_name
            )));
        }
        let fd = unsafe { libc::open(c"/dev/net/tun".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(Error::Resource(format!(
                "open /dev/net/tun: {}",
                io::Error::last_os_error()
            )));
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let mut ifr = [0u8; 40];
        let name = plan.tap_name.as_bytes();
        ifr[..name.len()].copy_from_slice(name);
        ifr[16..18].copy_from_slice(&(IFF_TAP | IFF_NO_PI).to_le_bytes());
        let rc = unsafe { libc::ioctl(file.as_raw_fd(), TUNSETIFF, ifr.as_mut_ptr()) };
        if rc < 0 {
            return Err(Error::Resource(format!(
                "TUNSETIFF {}: {}",
                plan.tap_name,
                io::Error::last_os_error()
            )));
        }
        let host = match plan.host_ip {
            std::net::IpAddr::V4(addr) => addr,
            std::net::IpAddr::V6(_) => {
                return Err(Error::Resource(
                    "TODO: assigning an IPv6 address to a tap device is not implemented".into(),
                ));
            }
        };
        configure_address(&plan.tap_name, host, plan.prefix)?;
        // The virtio-net worker polls the fd, so reads must not block the whole
        // device thread while the guest queue is short of buffers.
        set_nonblocking(file.as_raw_fd())?;
        Ok(Self {
            file,
            name: plan.tap_name.clone(),
        })
    }

    /// A second handle to the same tap interface, for the device worker thread.
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            file: self.file.try_clone()?,
            name: self.name.clone(),
        })
    }

    /// One Ethernet frame from the tap. Blocks until the fd is non-blocking.
    pub(crate) fn read_frame(&self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.file).read(buf)
    }

    /// One Ethernet frame to the tap.
    pub(crate) fn write_frame(&self, buf: &[u8]) -> io::Result<usize> {
        (&self.file).write(buf)
    }
}

fn set_nonblocking(fd: i32) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(Error::Resource(format!(
            "fcntl(F_GETFL) on tap: {}",
            io::Error::last_os_error()
        )));
    }
    let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if rc < 0 {
        return Err(Error::Resource(format!(
            "fcntl(F_SETFL) on tap: {}",
            io::Error::last_os_error()
        )));
    }
    Ok(())
}

fn configure_address(name: &str, addr: Ipv4Addr, prefix: u8) -> Result<()> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if sock < 0 {
        return Err(Error::Resource(format!(
            "socket for tap configuration: {}",
            io::Error::last_os_error()
        )));
    }
    let sock = unsafe { OwnedFd::from_raw_fd(sock) };
    ifreq_addr(name, addr, sock.as_raw_fd(), SIOCSIFADDR)?;
    ifreq_addr(name, prefix_mask(prefix), sock.as_raw_fd(), SIOCSIFNETMASK)?;
    let mut flags = [0u8; 40];
    flags[..name.len()].copy_from_slice(name.as_bytes());
    flags[16..18].copy_from_slice(&(IFF_UP | IFF_RUNNING).to_le_bytes());
    let rc = unsafe { libc::ioctl(sock.as_raw_fd(), SIOCSIFFLAGS, flags.as_mut_ptr()) };
    if rc < 0 {
        return Err(Error::Resource(format!(
            "SIOCSIFFLAGS {name}: {}",
            io::Error::last_os_error()
        )));
    }
    Ok(())
}

fn ifreq_addr(name: &str, addr: Ipv4Addr, fd: i32, request: libc::c_ulong) -> Result<()> {
    let mut ifr = [0u8; 40];
    ifr[..name.len()].copy_from_slice(name.as_bytes());
    // sockaddr_in starts at offset 16: family, port, address.
    ifr[16..18].copy_from_slice(&(libc::AF_INET as u16).to_le_bytes());
    ifr[20..24].copy_from_slice(&addr.octets());
    ioctl(fd, request, &mut ifr)
        .map_err(|err| Error::Resource(format!("ioctl {request:#x} on {name}: {err}")))
}

fn ioctl(fd: i32, request: libc::c_ulong, ifr: &mut [u8; 40]) -> io::Result<()> {
    let rc = unsafe { libc::ioctl(fd, request, ifr.as_mut_ptr()) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn prefix_mask(prefix: u8) -> Ipv4Addr {
    if prefix == 0 {
        return Ipv4Addr::UNSPECIFIED;
    }
    let mask = u32::MAX << (32 - prefix.min(32));
    Ipv4Addr::from(mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::NetworkPlan;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn open_reports_permission_or_succeeds() {
        let plan = NetworkPlan {
            tap_name: "cn0000aa".into(),
            host_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 127, 1)),
            guest_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 127, 2)),
            prefix: 24,
            gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 127, 1)),
            mac: [0x02, 0, 0, 0, 0, 2],
            leased_host: None,
        };
        match TapDevice::open(&plan) {
            Ok(dev) => assert_eq!(dev.name, "cn0000aa"),
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("Permission")
                        || msg.contains("permission")
                        || msg.contains("Operation not permitted")
                        || msg.contains("No such file or directory")
                        || msg.contains("tap"),
                    "{msg}"
                );
            }
        }
    }

    #[test]
    fn prefix_mask_24() {
        assert_eq!(prefix_mask(24), Ipv4Addr::new(255, 255, 255, 0));
    }
}
