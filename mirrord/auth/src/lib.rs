#![feature(result_option_inspect)]
#![feature(once_cell)]

use std::ops::Deref;

use serde::{Deserialize, Serialize};
use x509_certificate::{rfc2986, KeyAlgorithm, X509CertificateBuilder};

use crate::{certificate::Certificate, key_pair::KeyPair};

pub mod certificate;
pub mod key_pair;
pub mod license;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    certificate: Option<Certificate>,
    key_pair: KeyPair,
}

impl Credentials {
    pub fn certificate_request(&self, cn: &str) -> Result<rfc2986::CertificationRequest> {
        let mut builder = X509CertificateBuilder::new(KeyAlgorithm::Rsa);

        let _ = builder.subject().append_common_name_utf8_string(cn);

        builder
            .create_certificate_signing_request(self.key_pair.deref())
            .map_err(Box::from)
    }
}
