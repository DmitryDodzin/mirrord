use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct SecretToken(String);

impl SecretToken {
    pub fn secret(&self) -> &str {
        &self.0
    }
}

impl<T> From<T> for SecretToken
where
    T: AsRef<str>,
{
    fn from(val: T) -> Self {
        SecretToken(val.as_ref().to_owned())
    }
}

impl Debug for SecretToken {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "SecretToken(Redacted)")
    }
}
