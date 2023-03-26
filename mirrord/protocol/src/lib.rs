#![feature(const_trait_impl)]
#![feature(io_error_more)]
#![feature(result_option_inspect)]

pub mod codec;
pub mod dns;
pub mod error;
pub mod file;
pub mod outgoing;
pub mod proto;
pub mod std_types;
pub mod tcp;
pub mod prost {
    pub mod error {
        include!(concat!(env!("OUT_DIR"), "/protocol.error.rs"));
    }
    pub mod tcp {
        include!(concat!(env!("OUT_DIR"), "/protocol.tcp.rs"));
    }
    pub use super::std_types;
}

use std::{collections::HashSet, ops::Deref};

pub use codec::*;
pub use error::*;

pub type Port = u16;
pub type ConnectionId = u64;

/// A per-connection HTTP request ID
pub type RequestId = u16; // TODO: how many requests in a single connection? is u16 appropriate?

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnvVars(pub String);

impl From<EnvVars> for HashSet<String> {
    fn from(env_vars: EnvVars) -> Self {
        env_vars
            .split_terminator(';')
            .map(String::from)
            .collect::<HashSet<_>>()
    }
}

impl Deref for EnvVars {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
