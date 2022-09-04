use envconfig::Envconfig;
use mirrord_preview::filter::FilterPorts;

#[derive(Envconfig, Clone)]
pub struct LayerConfig {
    #[envconfig(from = "MIRRORD_AGENT_RUST_LOG", default = "info")]
    pub agent_rust_log: String,

    #[envconfig(from = "MIRRORD_AGENT_NAMESPACE", default = "default")]
    pub agent_namespace: String,

    #[envconfig(from = "MIRRORD_AGENT_IMAGE")]
    pub agent_image: Option<String>,

    #[envconfig(from = "MIRRORD_AGENT_IMAGE_PULL_POLICY", default = "IfNotPresent")]
    pub image_pull_policy: String,

    #[envconfig(from = "MIRRORD_AGENT_IMPERSONATED_POD_NAME")]
    pub impersonated_pod_name: Option<String>,

    #[envconfig(from = "MIRRORD_AGENT_IMPERSONATED_POD_NAMESPACE", default = "default")]
    pub impersonated_pod_namespace: String,

    #[envconfig(from = "MIRRORD_IMPERSONATED_CONTAINER_NAME")]
    pub impersonated_container_name: Option<String>,

    #[envconfig(from = "MIRRORD_ACCEPT_INVALID_CERTIFICATES", default = "false")]
    pub accept_invalid_certificates: bool,

    #[envconfig(from = "MIRRORD_AGENT_TTL", default = "0")]
    pub agent_ttl: u16,

    #[envconfig(from = "MIRRORD_AGENT_TCP_STEAL_TRAFFIC", default = "false")]
    pub agent_tcp_steal_traffic: bool,

    #[envconfig(from = "MIRRORD_AGENT_COMMUNICATION_TIMEOUT")]
    pub agent_communication_timeout: Option<u16>,

    #[envconfig(from = "MIRRORD_FILE_OPS", default = "false")]
    pub enabled_file_ops: bool,

    /// Filters out these env vars when overriding is enabled.
    #[envconfig(from = "MIRRORD_OVERRIDE_ENV_VARS_EXCLUDE", default = "")]
    pub override_env_vars_exclude: String,

    /// Selects these env vars when overriding is enabled.
    #[envconfig(from = "MIRRORD_OVERRIDE_ENV_VARS_INCLUDE", default = "")]
    pub override_env_vars_include: String,

    #[envconfig(from = "MIRRORD_EPHEMERAL_CONTAINER", default = "false")]
    pub ephemeral_container: bool,

    /// Enables resolving a remote DNS.
    #[envconfig(from = "MIRRORD_REMOTE_DNS", default = "false")]
    pub remote_dns: bool,

    #[envconfig(from = "MIRRORD_TCP_OUTGOING", default = "false")]
    pub enabled_tcp_outgoing: bool,

    #[envconfig(from = "MIRRORD_PREVIEW", default = "false")]
    pub preview: bool,

    #[envconfig(
        from = "MIRRORD_PREVIEW_SERVER",
        default = "https://layer.preview.metalbear.dev"
    )]
    pub preview_server: String,

    #[envconfig(
        from = "MIRRORD_PREVIEW_AUTH_SERVER",
        default = "https://identity.metalbear.dev"
    )]
    pub preview_auth_server: String,

    #[envconfig(from = "MIRRORD_PREVIEW_USERNAME")]
    pub preview_username: Option<String>,

    #[envconfig(from = "MIRRORD_PREVIEW_ALLOW_PORTS")]
    pub preview_allow_ports: Option<FilterPorts>,

    #[envconfig(from = "MIRRORD_PREVIEW_DENY_PORTS")]
    pub preview_deny_ports: Option<FilterPorts>,
}
