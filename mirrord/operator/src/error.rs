#[cfg(feature = "client")]
use bincode::{Decode, Encode};
use thiserror::Error;

#[derive(Debug, Error)]
#[cfg_attr(feature = "client", derive(Decode, Encode))]
pub enum OperatorError {
    #[error("Unable to lock port {0} for target {1}, currently locked by {2}")]
    LockedPort(u16, String, String),
    #[error("Deployment {0} doesn't have availabe pods")]
    DeploymentNoPods(String),
}
