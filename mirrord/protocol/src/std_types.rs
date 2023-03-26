include!(concat!(env!("OUT_DIR"), "/protocol.std_types.rs"));

impl From<std::net::IpAddr> for IpAddr {
    fn from(addr: std::net::IpAddr) -> Self {
        match addr {
            std::net::IpAddr::V4(addr) => addr.into(),
            std::net::IpAddr::V6(addr) => addr.into(),
        }
    }
}

impl From<std::net::Ipv4Addr> for IpAddr {
    fn from(addr: std::net::Ipv4Addr) -> Self {
        let inner = addr.octets().to_vec();

        IpAddr { inner }
    }
}

impl From<std::net::Ipv6Addr> for IpAddr {
    fn from(addr: std::net::Ipv6Addr) -> Self {
        let inner = addr.octets().to_vec();

        IpAddr { inner }
    }
}

impl From<IpAddr> for std::net::IpAddr {
    fn from(addr: IpAddr) -> Self {
        match addr.inner.len() {
            4 => {
                let addr: [u8; 4] = addr
                    .inner
                    .try_into()
                    .expect("couldn't unwrap even though lenght of vec is 4");

                std::net::IpAddr::from(addr)
            }
            16 => {
                let addr: [u8; 16] = addr
                    .inner
                    .try_into()
                    .expect("couldn't unwrap even though lenght of vec is 16");

                std::net::IpAddr::from(addr)
            }
            _ => unimplemented!(),
        }
    }
}

impl From<std::net::SocketAddr> for SocketAddr {
    fn from(addr: std::net::SocketAddr) -> Self {
        match addr {
            std::net::SocketAddr::V4(addr) => addr.into(),
            std::net::SocketAddr::V6(addr) => addr.into(),
        }
    }
}

impl From<std::net::SocketAddrV4> for SocketAddr {
    fn from(addr: std::net::SocketAddrV4) -> Self {
        SocketAddr {
            ip: Some(IpAddr::from(*addr.ip())),
            port: addr.port().into(),
            ..Default::default()
        }
    }
}

impl From<std::net::SocketAddrV6> for SocketAddr {
    fn from(addr: std::net::SocketAddrV6) -> Self {
        SocketAddr {
            ip: Some(IpAddr::from(*addr.ip())),
            port: addr.port().into(),
            flowinfo: Some(addr.flowinfo()),
            scope_id: Some(addr.scope_id()),
        }
    }
}
