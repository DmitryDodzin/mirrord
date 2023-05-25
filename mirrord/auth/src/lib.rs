#![feature(result_option_inspect)]
#![feature(once_cell)]

use std::{collections::BTreeMap, ops::Deref, path::PathBuf, sync::LazyLock};

use serde::{Deserialize, Serialize};
use tokio::fs;
use x509_certificate::{rfc2986, InMemorySigningKeyPair, KeyAlgorithm, X509CertificateBuilder};

use crate::{certificate::Certificate, key_pair::KeyPair};

pub mod certificate;
pub mod key_pair;
pub mod license;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

static CREDENTIALS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    home::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".mirrord/credentials")
});

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    certificate: Option<Certificate>,
    key_pair: KeyPair,
}

impl Credentials {
    pub fn init() -> Result<Self> {
        let key_algorithm = KeyAlgorithm::Ed25519;
        let (_, document) = InMemorySigningKeyPair::generate_random(key_algorithm)?;

        let pem_key = pem::Pem {
            tag: String::from("PRIVATE KEY"),
            contents: document.as_ref().to_vec(),
        };

        Ok(Credentials {
            certificate: None,
            key_pair: pem::encode(&pem_key).into(),
        })
    }

    pub fn is_ready(&self) -> bool {
        self.certificate.is_some()
    }

    pub fn certificate_request(&self, cn: &str) -> Result<rfc2986::CertificationRequest> {
        let mut builder = X509CertificateBuilder::new(KeyAlgorithm::Ed25519);

        let _ = builder.subject().append_common_name_utf8_string(cn);

        builder
            .create_certificate_signing_request(self.key_pair.deref())
            .map_err(Box::from)
    }
}

impl Deref for Credentials {
    type Target = Certificate;

    fn deref(&self) -> &Self::Target {
        self.certificate.as_ref().expect("Certificate not ready")
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CredentialStore {
    client_credentials: BTreeMap<String, Credentials>,
}

impl CredentialStore {
    pub async fn load() -> Result<Self> {
        let buffer = fs::read(&*CREDENTIALS_PATH).await?;

        serde_yaml::from_slice(&buffer).map_err(Box::from)
    }

    pub async fn save(&self) -> Result<()> {
        let buffer = serde_yaml::to_string(&self)?;

        fs::write(&*CREDENTIALS_PATH, &buffer)
            .await
            .map_err(Box::from)
    }

    pub fn get_or_init(&mut self, cluster_name: &str) -> Result<&mut Credentials> {
        if !self.client_credentials.contains_key(cluster_name) {
            self.client_credentials
                .insert(cluster_name.to_owned(), Credentials::init()?);
        }

        self.client_credentials
            .get_mut(cluster_name)
            .ok_or_else(|| unreachable!())
    }
}

#[cfg(test)]
mod tests {

    use serde_yaml::Value;
    use x509_certificate::CapturedX509Certificate;

    use super::*;
    use crate::license::License;

    #[tokio::test]
    async fn loading() -> Result<()> {
        let license_file = Value::Mapping(
            [
                (
                    Value::from("certificate"),
                    Value::from(include_str!("../cert/server.crt")),
                ),
                (
                    Value::from("key_pair"),
                    Value::from(include_str!("../cert/server.pk8")),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let license: License = serde_yaml::from_value(license_file)?;

        let mut store = CredentialStore::load().await.unwrap_or_default();

        let credentials = store.get_or_init("default")?;

        if !credentials.is_ready() {
            let request = credentials.certificate_request("foobar")?;

            credentials
                .certificate
                .replace(license.sign_certificate_request(request)?.into());
        }

        let cert = CapturedX509Certificate::from_der(credentials.encode_der()?)?;

        license.verify(&cert)?;

        Ok(())
    }
}
