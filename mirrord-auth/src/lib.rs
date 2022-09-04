#![feature(once_cell)]

#[cfg(feature = "webbrowser")]
use std::time::Duration;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[cfg(feature = "webbrowser")]
use rand::distributions::{Alphanumeric, DistString};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::secret::SecretToken;

mod secret;

static HOME_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    [
        std::env::var("HOME")
            .or_else(|_| std::env::var("HOMEPATH"))
            .unwrap_or_else(|_| "~".to_owned()),
        ".metalbear_credentials".to_owned(),
    ]
    .iter()
    .collect()
});

static AUTH_FILE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var("MIRRORD_AUTHENTICATION")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or_else(|| HOME_DIR.to_path_buf())
});

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthConfig {
    pub access_token: SecretToken,
    pub refresh_token: Option<SecretToken>,
}

#[derive(Error, Debug)]
pub enum AuthenticationError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ConfigParseError(#[from] serde_json::Error),
    #[error(transparent)]
    ConfigRequestError(#[from] reqwest::Error),
    #[error("missing refresh token in credentials")]
    MissingRefresh,
}

type Result<T> = std::result::Result<T, AuthenticationError>;

impl AuthConfig {
    pub fn config_path() -> &'static Path {
        AUTH_FILE_DIR.as_path()
    }

    pub fn load() -> Result<AuthConfig> {
        let bytes = fs::read(AUTH_FILE_DIR.as_path())?;

        serde_json::from_slice(&bytes).map_err(|err| err.into())
    }

    pub fn save(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)?;

        fs::write(AUTH_FILE_DIR.as_path(), bytes)?;

        Ok(())
    }

    pub fn refresh(&self, server: &str) -> Result<AuthConfig> {
        let refresh_token = self
            .refresh_token
            .as_ref()
            .ok_or(AuthenticationError::MissingRefresh)?;

        let client = reqwest::blocking::Client::new();

        client
            .post(format!("{}/oauth/refresh", server))
            .body(format!("\"{}\"", refresh_token.secret()))
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .send()?
            .error_for_status()?
            .json()
            .map_err(|err| err.into())
    }

    pub async fn refresh_async(&self, server: &str) -> Result<AuthConfig> {
        let refresh_token = self
            .refresh_token
            .as_ref()
            .ok_or(AuthenticationError::MissingRefresh)?;

        let client = reqwest::Client::new();

        client
            .post(format!("{}/oauth/refresh", server))
            .body(format!("\"{}\"", refresh_token.secret()))
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(|err| err.into())
    }

    pub fn verify(&self, server: &str) -> Result<()> {
        let client = reqwest::blocking::Client::new();

        client
            .get(format!("{}/oauth/verify", server))
            .bearer_auth(self.access_token.secret())
            .send()?
            .json()
            .map_err(|err| err.into())
    }

    pub async fn verify_async(&self, server: &str) -> Result<()> {
        let client = reqwest::Client::new();

        client
            .get(format!("{}/oauth/verify", server))
            .bearer_auth(self.access_token.secret())
            .send()
            .await?
            .json()
            .await
            .map_err(|err| err.into())
    }

    pub fn from_input(token: &str) -> Result<Self> {
        let mut parts = token.split(':');

        let access_token = parts.next().map(|val| val.into()).expect("Invalid Token");
        let refresh_token = parts.next().map(|val| val.into());

        Ok(AuthConfig {
            access_token,
            refresh_token,
        })
    }

    #[cfg(feature = "webbrowser")]
    pub fn from_webbrowser(server: &str, timeout: u64, no_open: bool) -> Result<AuthConfig> {
        let ref_id = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

        let url = format!("{}/oauth?ref={}", server, ref_id);

        let url_print = format!(
            "Please enter the following url in your webbrowser of choice\n\n url: {:?}\n",
            url
        );

        if no_open {
            println!("{}", url_print);
        } else if webbrowser::open(&url).is_err() {
            println!("Problem auto launching webbrowser\n{}", url_print);
        }

        let client = reqwest::blocking::Client::new();

        client
            .get(format!("{}/wait?ref={}", server, ref_id))
            .timeout(Duration::from_secs(timeout))
            .send()?
            .json()
            .map_err(|err| err.into())
    }
}
