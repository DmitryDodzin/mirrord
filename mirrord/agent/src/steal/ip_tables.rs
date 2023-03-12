use std::sync::LazyLock;

use fancy_regex::Regex;
use mirrord_protocol::Port;
use nix::unistd::getgid;
use rand::distributions::{Alphanumeric, DistString};
use tokio::process::Command;
use tracing::warn;

use crate::error::{AgentError, Result};

pub(crate) static MIRRORD_IPTABLE_PREROUTING_ENV: &str = "MIRRORD_IPTABLE_PREROUTING_NAME";
pub(crate) static MIRRORD_IPTABLE_OUTPUT_ENV: &str = "MIRRORD_IPTABLE_OUTPUT_NAME";

/// [`Regex`] used to select the `owner` rule from the list of `iptables` rules.
static UID_LOOKUP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-m owner --uid-owner \d+").unwrap());

static SKIP_PORTS_LOOKUP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-p tcp -m multiport --dports ([\d:,]+)").unwrap());

const IPTABLES_TABLE_NAME: &str = "nat";

#[cfg_attr(test, mockall::automock)]
pub(crate) trait IPTables {
    fn create_chain(&self, name: &str) -> Result<()>;
    fn remove_chain(&self, name: &str) -> Result<()>;

    fn add_rule(&self, chain: &str, rule: &str) -> Result<()>;
    fn insert_rule(&self, chain: &str, rule: &str, index: i32) -> Result<()>;
    fn list_rules(&self, chain: &str) -> Result<Vec<String>>;
    fn remove_rule(&self, chain: &str, rule: &str) -> Result<()>;
}

impl IPTables for iptables::IPTables {
    fn create_chain(&self, name: &str) -> Result<()> {
        self.new_chain(IPTABLES_TABLE_NAME, name)
            .map_err(|e| AgentError::IPTablesError(e.to_string()))?;
        self.append(IPTABLES_TABLE_NAME, name, "-j RETURN")
            .map_err(|e| AgentError::IPTablesError(e.to_string()))?;

        Ok(())
    }

    fn remove_chain(&self, name: &str) -> Result<()> {
        self.flush_chain(IPTABLES_TABLE_NAME, name)
            .map_err(|e| AgentError::IPTablesError(e.to_string()))?;
        self.delete_chain(IPTABLES_TABLE_NAME, name)
            .map_err(|e| AgentError::IPTablesError(e.to_string()))?;

        Ok(())
    }

    fn add_rule(&self, chain: &str, rule: &str) -> Result<()> {
        self.append(IPTABLES_TABLE_NAME, chain, rule)
            .map_err(|e| AgentError::IPTablesError(e.to_string()))
    }

    fn insert_rule(&self, chain: &str, rule: &str, index: i32) -> Result<()> {
        self.insert(IPTABLES_TABLE_NAME, chain, rule, index)
            .map_err(|e| AgentError::IPTablesError(e.to_string()))
    }

    fn list_rules(&self, chain: &str) -> Result<Vec<String>> {
        self.list(IPTABLES_TABLE_NAME, chain)
            .map_err(|e| AgentError::IPTablesError(e.to_string()))
    }

    fn remove_rule(&self, chain: &str, rule: &str) -> Result<()> {
        self.delete(IPTABLES_TABLE_NAME, chain, rule)
            .map_err(|e| AgentError::IPTablesError(e.to_string()))
    }
}

/// Wrapper struct for IPTables so it flushes on drop.
pub(crate) struct SafeIpTables<IPT: IPTables> {
    inner: IPT,
    chains: Vec<IpTableChain>,
    formatter: IPTableFormatter,
    flush_connections: bool,
}

/// Wrapper for using iptables. This creates a a new chain on creation and deletes it on drop.
/// The way it works is that it adds a chain, then adds a rule to the chain that returns to the
/// original chain (fallback) and adds a rule in the "PREROUTING" table that jumps to the new chain.
/// Connections will go then PREROUTING -> OUR_CHAIN -> IF MATCH REDIRECT -> IF NOT MATCH FALLBACK
/// -> ORIGINAL_CHAIN
impl<IPT> SafeIpTables<IPT>
where
    IPT: IPTables,
{
    pub(super) fn new(ipt: IPT, flush_connections: bool) -> Result<Self> {
        let formatter = IPTableFormatter::detect(&ipt)?;

        let chains = formatter.chains(&ipt);

        for chain in &chains {
            ipt.create_chain(&chain.name)?;

            if let Some(bypass) = formatter.bypass_own_packets_rule() {
                ipt.add_rule(&chain.name, &bypass);
            }

            let (entrypoint, entrypoint_rule) = chain.entrypoint();

            ipt.add_rule(entrypoint, entrypoint_rule)?
        }

        Ok(Self {
            inner: ipt,
            chains,
            formatter,
            flush_connections,
        })
    }

    /// Adds the redirect rule to iptables.
    ///
    /// Used to redirect packets when mirrord incoming feature is set to `steal`.
    #[tracing::instrument(level = "trace", skip(self))]
    pub(super) fn add_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        for chain in &self.chains {
            let (chain_name, rule) = chain.redirect_rule(redirected_port, target_port);

            self.inner.add_rule(chain_name, &rule)?;
        }

        Ok(())
    }

    /// Removes the redirect rule from iptables.
    ///
    /// Stops redirecting packets when mirrord incoming feature is set to `steal`, and there are no
    /// more subscribers on `target_port`.
    #[tracing::instrument(level = "trace", skip(self))]
    pub(super) fn remove_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        for chain in &self.chains {
            let (chain_name, rule) = chain.redirect_rule(redirected_port, target_port);

            self.inner.remove_rule(chain_name, &rule)?;
        }

        Ok(())
    }

    /// Adds port redirection, and bypass gid packets from iptables.
    #[tracing::instrument(level = "trace", skip(self))]
    pub(super) async fn add_stealer_iptables_rules(
        &self,
        redirected_port: Port,
        target_port: Port,
    ) -> Result<()> {
        self.add_redirect(redirected_port, target_port)?;

        if self.flush_connections {
            let conntrack = Command::new("conntrack")
                .args([
                    "--delete",
                    "--proto",
                    "tcp",
                    "--orig-port-dst",
                    &target_port.to_string(),
                ])
                .output()
                .await?;

            if !conntrack.status.success() && conntrack.status.code() != Some(256) {
                warn!("`conntrack` output is {conntrack:#?}");
            }
        }

        Ok(())
    }

    /// Removes port redirection, and bypass gid packets from iptables.
    #[tracing::instrument(level = "trace", skip(self))]
    pub(super) fn remove_stealer_iptables_rules(
        &self,
        redirected_port: Port,
        target_port: Port,
    ) -> Result<()> {
        self.remove_redirect(redirected_port, target_port)
    }
}

impl<IPT> Drop for SafeIpTables<IPT>
where
    IPT: IPTables,
{
    fn drop(&mut self) {
        for chain in &self.chains {
            let _ = chain.remove(&self.inner);
        }
    }
}

#[derive(Debug)]
pub(crate) enum IPTableFormatter {
    Normal,
    Mesh(String),
}

impl IPTableFormatter {
    const MESH_OUTPUTS: [&'static str; 2] = ["-j PROXY_INIT_OUTPUT", "-j ISTIO_OUTPUT"];
    const MESH_NAMES: [&'static str; 2] = ["PROXY_INIT_OUTPUT", "ISTIO_OUTPUT"];

    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn detect<IPT: IPTables>(ipt: &IPT) -> Result<Self> {
        let output = ipt.list_rules("OUTPUT")?;

        if let Some(mesh_output_chain) = output.iter().find_map(|rule| {
            IPTableFormatter::MESH_OUTPUTS
                .iter()
                .enumerate()
                .find_map(|(index, mesh_output)| {
                    rule.contains(mesh_output)
                        .then_some(IPTableFormatter::MESH_NAMES[index])
                })
        }) {
            warn!("skip_ports {skip_ports:?}");

            Ok(IPTableFormatter::Mesh(mesh_output_chain))
        } else {
            Ok(IPTableFormatter::Normal)
        }
    }

    fn chains<IPT: IPTables>(&self, ipt: &IPT) -> Vec<IpTableChain> {
        match self {
            IPTableFormatter::Normal => vec![IpTableChain::prerouting(None)],
            IPTableFormatter::Mesh(mesh_output_chain) => {
                let skip_ports = ipt
                    .list_rules("PROXY_INIT_REDIRECT")?
                    .iter()
                    .find_map(|rule| SKIP_PORTS_LOOKUP_REGEX.find(rule).ok().flatten())
                    .map(|m| m.as_str().to_string());

                let prerouting = IpTableChain::prerouting(skip_ports);

                // We extract --uid-owner value from the mesh's rules to get messages only from them
                // and not other processes sendning messages from localhost like healthprobe for
                // grpc. This to more closely match behavior with non meshed
                // services
                let filter = ipt
                    .list_rules(mesh_output_chain)?
                    .iter()
                    .find_map(|rule| UID_LOOKUP_REGEX.find(rule).ok().flatten())
                    .map(|m| format!("-o lo {}", m.as_str())).or_else(|| {
                        warn!("Couldn't find --uid-owner of meshed chain {mesh_ipt_chain:?} falling back on \"-o lo\" rule");

                        Some("-o lo".to_owned())
                    });

                let output = IpTableChain::output(filter);

                vec![prerouting, output]
            }
        }
    }

    /// Adds a `RETURN` rule based on `gid` to iptables.
    ///
    /// When the mirrord incoming feature is set to `steal`, and we're using a filter (instead of
    /// stealing every packet), we need this rule to avoid stealing our own packets, that were sent
    /// to their original destinations.
    fn bypass_own_packets_rule(&self) -> Option<String> {
        match self {
            IPTableFormatter::Normal => None,
            IPTableFormatter::Mesh(_) => {
                let gid = getgid();
                Some(format!("-m owner --gid-owner {gid} -p tcp -j RETURN"))
            }
        }
    }
}

#[derive(Debug)]
pub struct IpTableChain {
    name: String,
    entrypoint_name: &'static str,
    entrypoint_rule: String,
    redirect_filter: Option<String>,
}

impl IpTableChain {
    fn prerouting(entrypoint_rule: Option<String>) -> Self {
        let chain_name = std::env::var(MIRRORD_IPTABLE_PREROUTING_ENV).unwrap_or_else(|_| {
            format!(
                "MIRRORD_PREROUTING_REDIRECT_{}",
                Alphanumeric.sample_string(&mut rand::thread_rng(), 5)
            )
        });

        let entrypoint_rule = entrypoint_rule
            .map(|rule| format!("{rule} -j {chain_name}"))
            .unwrap_or_else(|| format!("-j {chain_name}"));

        IpTableChain {
            entrypoint_name: "PREROUTING",
            chain_name,
            entrypoint_rule,
            redirect_filter: None,
        }
    }

    fn output(redirect_filter: Option<String>) -> Self {
        let chain_name = std::env::var(MIRRORD_IPTABLE_OUTPUT_ENV).unwrap_or_else(|_| {
            format!(
                "MIRRORD_OUTPUT_REDIRECT_{}",
                Alphanumeric.sample_string(&mut rand::thread_rng(), 5)
            )
        });

        RuleSet {
            entrypoint_name: "OUTPUT",
            chain_name,
            entrypoint_rule: format!("-j {chain_name}"),
            redirect_filter,
        }
    }

    fn entrypoint(&self) -> (&str, &str) {
        (self.entrypoint_name, &self.entrypoint_rule)
    }

    fn redirect(&self, redirected_port: Port, target_port: Port) -> (&str, String) {
        let redirect_rule =
            format!("-m tcp -p tcp --dport {redirected_port} -j REDIRECT --to-ports {target_port}");

        let redirect_rule = match &self.redirect_filter {
            Some(filter) => format!("{filter} {redirect_rule}"),
            None => redirect_rule,
        };

        (&self.chain_name, redirect_rule)
    }

    fn remove<IPT: IPTables>(&self, ipt: &IPT) -> Result<()> {
        ipt.remove_rule(self.entrypoint_name, &self.entrypoint_rule)?;

        ipt.remove_chain(self.chain_name)
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::*;

    use super::*;

    #[test]
    fn default() {
        let mut mock = MockIPTables::new();

        mock.expect_list_rules()
            .with(eq("OUTPUT"))
            .returning(|_| Ok(vec![]));

        mock.expect_create_chain()
            .with(str::starts_with("MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_remove_chain()
            .with(str::starts_with("MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_add_rule()
            .with(eq("PREROUTING"), str::starts_with("-j MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_, _| Ok(()));

        mock.expect_insert_rule()
            .with(
                str::starts_with("MIRRORD_REDIRECT_"),
                eq("-m tcp -p tcp --dport 69 -j REDIRECT --to-ports 420"),
                eq(1),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        mock.expect_remove_rule()
            .with(eq("PREROUTING"), str::starts_with("-j MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_, _| Ok(()));

        mock.expect_remove_rule()
            .with(
                str::starts_with("MIRRORD_REDIRECT_"),
                eq("-m tcp -p tcp --dport 69 -j REDIRECT --to-ports 420"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let ipt = SafeIpTables::new(mock, false).expect("Create Failed");

        assert!(ipt.add_redirect(69, 420).is_ok());

        assert!(ipt.remove_redirect(69, 420).is_ok());
    }

    #[test]
    fn linkerd() {
        let mut mock = MockIPTables::new();

        mock.expect_list_rules()
            .with(eq("OUTPUT"))
            .returning(|_| Ok(vec!["-j PROXY_INIT_OUTPUT".to_owned()]));

        mock.expect_list_rules()
            .with(eq("PROXY_INIT_OUTPUT"))
            .returning(|_| {
                Ok(vec![
                    "-N PROXY_INIT_OUTPUT".to_owned(),
                    "-A PROXY_INIT_OUTPUT -m owner --uid-owner 2102 -m comment --comment
    \"proxy-init/ignore-proxy-user-id/1676542558\" -j RETURN"
                        .to_owned(),
                    "-A
    PROXY_INIT_OUTPUT -o lo -m comment --comment \"proxy-init/ignore-loopback/1676542558\" -j
    RETURN"
                        .to_owned(),
                ])
            });

        mock.expect_create_chain()
            .with(str::starts_with("MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_remove_chain()
            .with(str::starts_with("MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_add_rule()
            .with(eq("OUTPUT"), str::starts_with("-j MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_, _| Ok(()));

        mock.expect_insert_rule()
            .with(
                str::starts_with("MIRRORD_REDIRECT_"),
                eq(
                    "-o lo -m owner --uid-owner 2102 -m tcp -p tcp --dport 69 -j REDIRECT
    --to-ports 420",
                ),
                eq(1),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        mock.expect_remove_rule()
            .with(eq("OUTPUT"), str::starts_with("-j MIRRORD_REDIRECT_"))
            .times(1)
            .returning(|_, _| Ok(()));

        mock.expect_remove_rule()
            .with(
                str::starts_with("MIRRORD_REDIRECT_"),
                eq(
                    "-o lo -m owner --uid-owner 2102 -m tcp -p tcp --dport 69 -j REDIRECT
    --to-ports 420",
                ),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let ipt = SafeIpTables::new(mock, false).expect("Create Failed");

        assert!(ipt.add_redirect(69, 420).is_ok());

        assert!(ipt.remove_redirect(69, 420).is_ok());
    }
}
