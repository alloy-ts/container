use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::{Error, Result};
use crate::types::{NetworkConfig, NetworkMode};

/// One dedicated microVM NIC. `prefix` is the CIDR prefix length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkPlan {
    pub tap_name: String,
    pub host_ip: IpAddr,
    pub guest_ip: IpAddr,
    pub prefix: u8,
    pub gateway: IpAddr,
    pub mac: [u8; 6],
}

const HOST: Ipv4Addr = Ipv4Addr::new(192, 168, 127, 1);

/// Next guest host address in 192.168.127.0/24. .0 is the subnet, .1 is the gateway.
static NEXT_HOST: AtomicU32 = AtomicU32::new(2);

pub(crate) fn plan(id: &str, config: &NetworkConfig) -> Result<Option<NetworkPlan>> {
    match config.mode {
        NetworkMode::None => Ok(None),
        NetworkMode::Slirp => Err(Error::Resource(
            "TODO: slirp/user-mode networking is not implemented; use NetworkMode::Dedicated or None"
                .into(),
        )),
        NetworkMode::Dedicated => Ok(Some(dedicated(id, config)?)),
    }
}

fn dedicated(id: &str, config: &NetworkConfig) -> Result<NetworkPlan> {
    let guest_ip = match config.guest_ip {
        Some(IpAddr::V4(addr)) => addr,
        Some(IpAddr::V6(_)) => {
            return Err(Error::Resource(
                "TODO: IPv6 tap addressing is not implemented".into(),
            ));
        }
        None => {
            let host = NEXT_HOST.fetch_add(1, Ordering::Relaxed);
            if host >= 254 {
                return Err(Error::Resource(
                    "dedicated IPv4 pool 192.168.127.0/24 is exhausted".into(),
                ));
            }
            Ipv4Addr::new(192, 168, 127, host as u8)
        }
    };
    let gateway = match config.gateway {
        Some(IpAddr::V4(addr)) => addr,
        Some(IpAddr::V6(_)) => {
            return Err(Error::Resource(
                "TODO: IPv6 gateways are not implemented".into(),
            ));
        }
        None => HOST,
    };
    let mac = match &config.mac {
        Some(text) => parse_mac(text)?,
        None => mac_from_id(id, guest_ip),
    };
    Ok(NetworkPlan {
        tap_name: tap_name(id),
        host_ip: IpAddr::V4(HOST),
        guest_ip: IpAddr::V4(guest_ip),
        prefix: 24,
        gateway: IpAddr::V4(gateway),
        mac,
    })
}

fn tap_name(id: &str) -> String {
    // IFNAMSIZ is 16 including NUL. `cn` + 6 hex chars fits.
    let mut hash = 0x811c9dc5u32;
    for byte in id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    format!("cn{:06x}", hash & 0x00ff_ffff)
}

fn mac_from_id(id: &str, guest: Ipv4Addr) -> [u8; 6] {
    let mut hash = 0u32;
    for byte in id.as_bytes() {
        hash = hash.wrapping_mul(16777619) ^ u32::from(*byte);
    }
    let octets = guest.octets();
    // Locally administered, unicast.
    [0x02, 0x43, 0x4e, hash as u8, octets[2], octets[3]]
}

fn parse_mac(text: &str) -> Result<[u8; 6]> {
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() != 6 {
        return Err(Error::Resource(format!("invalid mac address {text}")));
    }
    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16)
            .map_err(|_| Error::Resource(format!("invalid mac address {text}")))?;
    }
    Ok(mac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedicated_plan_assigns_a_guest_address() {
        let plan = plan("alpha", &NetworkConfig::dedicated()).unwrap().unwrap();
        assert!(plan.tap_name.starts_with("cn"));
        assert!(plan.tap_name.len() <= 15);
        assert_eq!(plan.prefix, 24);
        assert_eq!(plan.mac[0], 0x02);
        assert!(matches!(plan.guest_ip, IpAddr::V4(_)));
    }

    #[test]
    fn none_plans_no_nic() {
        assert!(plan("alpha", &NetworkConfig::none()).unwrap().is_none());
    }

    #[test]
    fn honors_an_explicit_address() {
        let mut cfg = NetworkConfig::dedicated();
        cfg.guest_ip = Some(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)));
        cfg.mac = Some("02:00:00:00:00:0a".into());
        let plan = plan("beta", &cfg).unwrap().unwrap();
        assert_eq!(plan.guest_ip, IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)));
        assert_eq!(plan.mac, [0x02, 0, 0, 0, 0, 0x0a]);
    }
}
