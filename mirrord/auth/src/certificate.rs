use std::ops::Deref;

use serde::{de, ser, Deserialize, Serialize};
use x509_certificate::X509Certificate;

fn x509_serialize<S>(certificate: &X509Certificate, serialzer: S) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    let certificate = certificate.encode_pem().map_err(ser::Error::custom)?;

    certificate.serialize(serialzer)
}

fn x509_deserialize<'de, D>(deserializer: D) -> Result<X509Certificate, D::Error>
where
    D: de::Deserializer<'de>,
{
    let certificate = String::deserialize(deserializer)?;

    X509Certificate::from_pem(certificate).map_err(de::Error::custom)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Certificate(
    #[serde(
        deserialize_with = "x509_deserialize",
        serialize_with = "x509_serialize"
    )]
    X509Certificate,
);

impl From<X509Certificate> for Certificate {
    fn from(certificate: X509Certificate) -> Self {
        Certificate(certificate)
    }
}

impl Deref for Certificate {
    type Target = X509Certificate;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
