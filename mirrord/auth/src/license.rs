use bcder::{encode::Values as _, BitString, Mode};
use bytes::Bytes;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use x509_certificate::{
    asn1time::Time, rfc2986, rfc5280, CapturedX509Certificate, Sign as _, Signer as _,
    X509Certificate, X509CertificateError,
};

use crate::{certificate::Certificate, key_pair::KeyPair};

type Result<T, E = X509CertificateError> = std::result::Result<T, E>;

#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    certificate: Certificate,
    key_pair: KeyPair,
}

impl License {
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
        certificate.verify_signed_by_certificate(self)
    }
}

impl AsRef<X509Certificate> for License {
    fn as_ref(&self) -> &X509Certificate {
        &self.certificate
    }
}

unsafe impl Send for License {}
unsafe impl Sync for License {}
