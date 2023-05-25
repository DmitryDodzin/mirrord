#![feature(result_option_inspect)]
#![feature(once_cell)]

use std::{collections::BTreeMap, fmt::Debug, ops::Deref, path::PathBuf, sync::LazyLock};

use kube::{api::PostParams, Api, Client, Resource};
use serde::{Deserialize, Serialize};
use tokio::fs;
use x509_certificate::{rfc2986, InMemorySigningKeyPair, KeyAlgorithm, X509CertificateBuilder};

use crate::{certificate::Certificate, key_pair::KeyPair};

pub mod certificate;
pub mod key_pair;
pub mod license;

pub type AuthenticationError = Box<dyn std::error::Error + Send + Sync>;

type Result<T, E = AuthenticationError> = std::result::Result<T, E>;

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

    pub async fn get_client_certificate<R>(&mut self, client: Client, cn: &str) -> Result<()>
    where
        R: Resource + Clone + Debug,
        R: for<'de> Deserialize<'de>,
        R::DynamicType: Default,
    {
        let certificate_request = self.certificate_request(cn)?.encode_pem()?;

        let api: Api<R> = Api::all(client);

        let certificate: Certificate = api
            .create_subresource(
                "certificate",
                "operator",
                &PostParams::default(),
                certificate_request.into(),
            )
            .await?;

        self.certificate.replace(certificate);

        Ok(())
    }
}

impl Deref for Credentials {
    type Target = Certificate;

    fn deref(&self) -> &Self::Target {
        self.certificate.as_ref().expect("Certificate not ready")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialStore {
    active: String,
    client_credentials: BTreeMap<String, Credentials>,
}

impl Default for CredentialStore {
    fn default() -> Self {
        CredentialStore {
            active: "default".to_string(),
            client_credentials: BTreeMap::new(),
        }
    }
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

    pub async fn get_or_init<R>(&mut self, client: &Client, cn: &str) -> Result<&mut Credentials>
    where
        R: Resource + Clone + Debug,
        R: for<'de> Deserialize<'de>,
        R::DynamicType: Default,
    {
        if !self.client_credentials.contains_key(&self.active) {
            self.client_credentials
                .insert(self.active.clone(), Credentials::init()?);
        }

        let credentials = self
            .client_credentials
            .get_mut(&self.active)
            .expect("Unreachable");

        if !credentials.is_ready() {
            credentials
                .get_client_certificate::<R>(client.clone(), cn)
                .await?;
        }

        Ok(credentials)
    }
}
