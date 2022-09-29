use mirrord_config_derive::MirrordConfig;
use serde::Deserialize;

use crate::{
    config::{
        default_value::DefaultValue, from_env::FromEnv, source::MirrordConfigSource, ConfigError,
        MirrordConfig,
    },
    util::{MirrordToggleableConfig, VecOrSingle},
};

#[derive(MirrordConfig, Default, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[config(map_to = PreviewConfig)]
pub struct PreviewFileConfig {
    #[config(env = "MIRRORD_PREVIEW", default = "false")]
    pub enabled: Option<bool>,

    #[config(
        env = "MIRRORD_PREVIEW_SERVER",
        default = "https://layer.preview.metalbear.dev"
    )]
    pub server: Option<String>,

    #[config(
        env = "MIRRORD_PREVIEW_AUTH_SERVER",
        default = "https://identity.metalbear.dev"
    )]
    pub auth_server: Option<String>,

    #[config(env = "MIRRORD_PREVIEW_USERNAME")]
    pub username: Option<String>,

    #[config(env = "MIRRORD_PREVIEW_ALLOW_PORTS")]
    pub allow_ports: Option<VecOrSingle<String>>,

    #[config(env = "MIRRORD_PREVIEW_DENY_PORTS")]
    pub deny_ports: Option<VecOrSingle<String>>,
}

impl MirrordToggleableConfig for PreviewFileConfig {
    fn enabled_config() -> Result<Self::Generated, ConfigError> {
        let enabled = (FromEnv::new("MIRRORD_PREVIEW"), DefaultValue::new("true"))
            .source_value()
            .ok_or(ConfigError::ValueNotProvided(
                "PreviewFileConfig",
                "enabled",
                Some("MIRRORD_TCP_OUTGOING"),
            ))?;

        Self::default()
            .generate_config()
            .map(|generated| PreviewConfig {
                enabled,
                ..generated
            })
    }
    fn disabled_config() -> Result<Self::Generated, ConfigError> {
        Self::default().generate_config()
    }
}
