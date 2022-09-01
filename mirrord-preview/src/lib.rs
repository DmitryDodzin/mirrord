#[cfg(feature = "client")]
pub mod client;
pub mod connection;
pub mod filter;
pub mod proxy;

#[derive(Debug)]
pub struct PreviewConfig {
    pub server: String,
    pub username: Option<String>,
    pub allow_ports: Option<filter::FilterPorts>,
    pub deny_ports: filter::FilterPorts,
}
