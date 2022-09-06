#[cfg(feature = "client")]
pub mod client;
pub mod codec;
pub mod connection;
pub mod filter;
pub mod proxy;

#[derive(Debug)]
pub struct PreviewConfig {
    pub server: String,
    pub auth_server: String,
    pub username: Option<String>,
    pub allow_ports: Option<filter::FilterPorts>,
    pub deny_ports: filter::FilterPorts,
    pub listen_for_updates: bool,
}

pub type RequestCodec = codec::BincodeCodec<proxy::ProxiedRequest>;
pub type ResponseCodec = codec::BincodeCodec<proxy::ProxiedResponse>;
