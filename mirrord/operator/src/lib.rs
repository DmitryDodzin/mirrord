#![feature(once_cell)]

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub mod crd;

pub mod license;

/// Operator Setup functinality
#[cfg(feature = "setup")]
pub mod setup;

#[cfg(feature = "protocol")]
pub mod protocol;
