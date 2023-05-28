use std::{path::Path, str::FromStr};

use base64::{engine::general_purpose, Engine as _};
use bcder::{encode::Values as _, BitString, Mode};
use bytes::Bytes;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;
use x509_certificate::{
    asn1time::Time, rfc2986, rfc5280, CapturedX509Certificate, Sign as _, Signer as _,
    X509Certificate,
};

use crate::{
    certificate::Certificate,
    error::{AuthenticationError, Result},
    key_pair::KeyPair,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    certificate: Certificate,
    key_pair: KeyPair,
}

impl License {
    pub async fn from_paths<C, K>(certificate_path: C, key_pair_path: K) -> Result<Self>
    where
        C: AsRef<Path>,
        K: AsRef<Path>,
    {
        let certificate = fs::read_to_string(certificate_path).await?.parse()?;
        let key_pair = fs::read_to_string(key_pair_path).await?.into();

        Ok(License {
            certificate,
            key_pair,
        })
    }

    pub fn sign_certificate_request(
        &self,
        request: rfc2986::CertificationRequest,
    ) -> Result<X509Certificate> {
        let tbs_certificate = rfc5280::TbsCertificate {
            version: Some(rfc5280::Version::V3),
            serial_number: 1.into(),
            signature: self.certificate.signature_algorithm().unwrap().into(),
            issuer: self.certificate.subject_name().clone(),
            validity: rfc5280::Validity {
                not_before: Time::from(Utc::now()),
                not_after: Time::from(Utc::now() + Duration::days(365)),
            },
            subject: request.certificate_request_info.subject,
            subject_public_key_info: request.certificate_request_info.subject_public_key_info,
            issuer_unique_id: None,
            subject_unique_id: None,
            extensions: None,
            raw_data: None,
        };

        let mut tbs_der = Vec::<u8>::new();
        tbs_certificate
            .encode_ref()
            .write_encoded(Mode::Der, &mut tbs_der)?;

        let signature = self.key_pair.try_sign(&tbs_der)?;
        let signature_algorithm = self.key_pair.signature_algorithm()?;

        let cert = rfc5280::Certificate {
            tbs_certificate,
            signature_algorithm: signature_algorithm.into(),
            signature: BitString::new(0, Bytes::copy_from_slice(signature.as_ref())),
        };

        Ok(X509Certificate::from(cert))
    }

    pub fn verify(&self, certificate: &CapturedX509Certificate) -> Result<()> {
        certificate
            .verify_signed_by_certificate(self)
            .map_err(AuthenticationError::from)
    }

    pub fn info(&self) -> LicenseInfo<'_> {
        LicenseInfo(self)
    }
}

impl AsRef<X509Certificate> for License {
    fn as_ref(&self) -> &X509Certificate {
        &self.certificate
    }
}

impl FromStr for License {
    type Err = AuthenticationError;

    fn from_str(encoded: &str) -> Result<Self, Self::Err> {
        let decoded = general_purpose::STANDARD.decode(encoded)?;

        let mut certificate = None;
        let mut key_pair = None;

        for pem in pem::parse_many(decoded)? {
            match pem.tag() {
                "CERTIFICATE" => {
                    let x509 = X509Certificate::from_der(pem.contents())?;
                    certificate = Some(Certificate::from(x509));
                }
                "PRIVATE KEY" => key_pair = Some(KeyPair::from(pem::encode(&pem))),
                _ => {}
            }
        }

        match (certificate, key_pair) {
            (Some(certificate), Some(key_pair)) => Ok(License {
                certificate,
                key_pair,
            }),
            _ => todo!("Missing certificates"),
        }
    }
}

unsafe impl Send for License {}
unsafe impl Sync for License {}

#[derive(Debug)]
pub struct LicenseInfo<'l>(&'l License);

impl<'l> LicenseInfo<'l> {
    pub fn name(&self) -> String {
        self.0
            .certificate
            .subject_common_name()
            .unwrap_or_else(|| "No Name".to_string())
    }

    pub fn organization(&self) -> String {
        self.0
            .certificate
            .subject_name()
            .iter_organization()
            .filter_map(|org| org.to_string().ok())
            .next()
            .unwrap_or_else(|| "No Organization".to_string())
    }
}
