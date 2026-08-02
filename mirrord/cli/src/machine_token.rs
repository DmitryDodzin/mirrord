use std::env;

use mirrord_auth::credentials::MachineToken;

use crate::ci::error::CiError;

/// Env var that carries the machine token, which the user has to set in order to execute the
/// `mirrord ci`/`mirrord cloud-agent` commands when the operator is available.
///
/// Should be set to the value they got from [`generate_ci_api_key`].
///
/// Sessions authenticated with a machine token count against the operator's concurrent machine
/// session cap instead of consuming a developer seat, which is what makes it the right credential
/// for both CI pipelines and cloud agents.
pub(crate) const MIRRORD_MACHINE_TOKEN: &str = "MIRRORD_MACHINE_TOKEN";

/// Previous name of [`MIRRORD_MACHINE_TOKEN`], from when CI was the only machine flow.
///
/// Still accepted so that existing CI setups keep working without changes.
pub(crate) const MIRRORD_CI_API_KEY: &str = "MIRRORD_CI_API_KEY";

/// Env vars that may carry the machine token, in the order we check them.
///
/// [`MIRRORD_MACHINE_TOKEN`] wins when both are set.
const MACHINE_TOKEN_ENV_VARS: [&str; 2] = [MIRRORD_MACHINE_TOKEN, MIRRORD_CI_API_KEY];

/// [`ci_api_key_available`] with the env lookup injected, so the resolution order can be tested
/// without mutating the process environment.
///
/// A token that is set but malformed is an error rather than a missing token: silently falling
/// back would run the session against a developer seat, which is the opposite of what the user
/// asked for by setting the var.
pub(crate) fn machine_token_from<F>(mut get_var: F) -> Result<Option<MachineToken>, CiError>
where
    F: FnMut(&'static str) -> Result<String, env::VarError>,
{
    for env_var in MACHINE_TOKEN_ENV_VARS {
        match get_var(env_var) {
            Ok(api_key) => return Ok(Some(MachineToken::decode(&api_key)?)),
            Err(env::VarError::NotPresent) => continue,
            Err(fail @ env::VarError::NotUnicode(..)) => {
                return Err(CiError::EnvVar(env_var, fail));
            }
        }
    }

    Ok(None)
}
