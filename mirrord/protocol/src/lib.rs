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
        use std::io;

        include!(concat!(env!("OUT_DIR"), "/protocol.error.rs"));

        impl From<io::ErrorKind> for ErrorKindInternal {
            fn from(kind: io::ErrorKind) -> Self {
                let mut unknown_value = None;

                let kind = match kind {
                    io::ErrorKind::NotFound => ErrorKind::NotFound,
                    io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
                    io::ErrorKind::ConnectionRefused => ErrorKind::ConnectionRefused,
                    io::ErrorKind::ConnectionReset => ErrorKind::ConnectionReset,
                    io::ErrorKind::HostUnreachable => ErrorKind::HostUnreachable,
                    io::ErrorKind::NetworkUnreachable => ErrorKind::NetworkUnreachable,
                    io::ErrorKind::ConnectionAborted => ErrorKind::ConnectionAborted,
                    io::ErrorKind::NotConnected => ErrorKind::NotConnected,
                    io::ErrorKind::AddrInUse => ErrorKind::AddrInUse,
                    io::ErrorKind::AddrNotAvailable => ErrorKind::AddrNotAvailable,
                    io::ErrorKind::NetworkDown => ErrorKind::NetworkDown,
                    io::ErrorKind::BrokenPipe => ErrorKind::BrokenPipe,
                    io::ErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
                    io::ErrorKind::WouldBlock => ErrorKind::WouldBlock,
                    io::ErrorKind::NotADirectory => ErrorKind::NotADirectory,
                    io::ErrorKind::IsADirectory => ErrorKind::IsADirectory,
                    io::ErrorKind::DirectoryNotEmpty => ErrorKind::DirectoryNotEmpty,
                    io::ErrorKind::ReadOnlyFilesystem => ErrorKind::ReadOnlyFilesystem,
                    io::ErrorKind::FilesystemLoop => ErrorKind::FilesystemLoop,
                    io::ErrorKind::StaleNetworkFileHandle => ErrorKind::StaleNetworkFileHandle,
                    io::ErrorKind::InvalidInput => ErrorKind::InvalidInput,
                    io::ErrorKind::InvalidData => ErrorKind::InvalidData,
                    io::ErrorKind::TimedOut => ErrorKind::TimedOut,
                    io::ErrorKind::WriteZero => ErrorKind::WriteZero,
                    io::ErrorKind::StorageFull => ErrorKind::StorageFull,
                    io::ErrorKind::NotSeekable => ErrorKind::NotSeekable,
                    io::ErrorKind::FilesystemQuotaExceeded => ErrorKind::FilesystemQuotaExceeded,
                    io::ErrorKind::FileTooLarge => ErrorKind::FileTooLarge,
                    io::ErrorKind::ResourceBusy => ErrorKind::ResourceBusy,
                    io::ErrorKind::ExecutableFileBusy => ErrorKind::ExecutableFileBusy,
                    io::ErrorKind::Deadlock => ErrorKind::Deadlock,
                    io::ErrorKind::CrossesDevices => ErrorKind::CrossesDevices,
                    io::ErrorKind::TooManyLinks => ErrorKind::TooManyLinks,
                    io::ErrorKind::InvalidFilename => ErrorKind::InvalidFilename,
                    io::ErrorKind::ArgumentListTooLong => ErrorKind::ArgumentListTooLong,
                    io::ErrorKind::Interrupted => ErrorKind::Interrupted,
                    io::ErrorKind::Unsupported => ErrorKind::Unsupported,
                    io::ErrorKind::UnexpectedEof => ErrorKind::UnexpectedEof,
                    io::ErrorKind::OutOfMemory => ErrorKind::OutOfMemory,
                    io::ErrorKind::Other => ErrorKind::Other,
                    _ => {
                        unknown_value = Some(kind.to_string());
                        ErrorKind::Unknown
                    }
                }
                .into();

                ErrorKindInternal {
                    kind,
                    unknown_value,
                }
            }
        }

        impl From<RemoteError> for ResponseError {
            fn from(remote_error: RemoteError) -> Self {
                ResponseError {
                    response_error_type: Some(response_error::ResponseErrorType::RemoteError(
                        remote_error,
                    )),
                }
            }
        }
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
