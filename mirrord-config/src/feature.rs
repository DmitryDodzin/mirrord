use mirrord_config_derive::MirrordConfig;
use serde::Deserialize;

use crate::{
    config::source::MirrordConfigSource, env::EnvFileConfig, fs::FsConfig,
    network::NetworkFileConfig, preview::PreviewFileConfig, util::ToggleableConfig,
};

#[derive(MirrordConfig, Deserialize, PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[config(map_to = FeatureConfig)]
pub struct FeatureFileConfig {
    #[serde(default)]
    #[config(nested)]
    pub env: ToggleableConfig<EnvFileConfig>,

    #[serde(default)]
    #[config(nested)]
    pub fs: ToggleableConfig<FsConfig>,

    #[serde(default)]
    #[config(nested)]
    pub network: ToggleableConfig<NetworkFileConfig>,

    #[serde(default = "ToggleableConfig::disabled")]
    #[config(nested)]
    pub preview: ToggleableConfig<PreviewFileConfig>,
}

impl Default for FeatureFileConfig {
    fn default() -> Self {
        FeatureFileConfig {
            env: ToggleableConfig::enabled(),
            fs: ToggleableConfig::enabled(),
            network: ToggleableConfig::enabled(),
            preview: ToggleableConfig::disabled(),
        }
    }
}
