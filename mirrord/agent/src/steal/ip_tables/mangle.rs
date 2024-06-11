use std::sync::Arc;

use async_trait::async_trait;
use mirrord_protocol::Port;
use nix::unistd::getgid;
use tracing::warn;

use crate::{
    error::Result,
    steal::ip_tables::{chain::IPTableChain, IPTables, Redirect},
};

pub(crate) struct MangleRedirect<IPT: IPTables> {
    pub(crate) managed: IPTableChain<IPT>,
}

impl<IPT> MangleRedirect<IPT>
where
    IPT: IPTables,
{
    const ENTRYPOINT: &'static str = "PREROUTING";

    pub fn create(ipt: Arc<IPT>, chain_name: String) -> Result<Self> {
        let managed = IPTableChain::create(ipt.with_table("filter").into(), chain_name)?;

        let gid = getgid();
        managed
            .add_rule(&format!("-m owner --gid-owner {gid} -p tcp -j RETURN"))
            .inspect_err(|_| {
                warn!("Unable to create iptable rule with \"--gid-owner {gid}\" filter")
            })?;

        Ok(MangleRedirect { managed })
    }

    pub fn load(ipt: Arc<IPT>, chain_name: String) -> Result<Self> {
        let managed = IPTableChain::create(ipt.with_table("filter").into(), chain_name)?;

        Ok(MangleRedirect { managed })
    }
}

#[async_trait]
impl<IPT> Redirect for MangleRedirect<IPT>
where
    IPT: IPTables + Send + Sync,
{
    async fn mount_entrypoint(&self) -> Result<()> {
        self.managed.inner().add_rule(
            Self::ENTRYPOINT,
            &format!("-j {}", self.managed.chain_name()),
        )?;

        Ok(())
    }

    async fn unmount_entrypoint(&self) -> Result<()> {
        self.managed.inner().remove_rule(
            Self::ENTRYPOINT,
            &format!("-j {}", self.managed.chain_name()),
        )?;

        Ok(())
    }

    async fn add_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        let redirect_rule =
            format!("-m tcp -p tcp --dport {redirected_port} -j REDIRECT --to-ports {target_port}");

        self.managed.add_rule(&redirect_rule)?;

        Ok(())
    }

    async fn remove_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        let redirect_rule =
            format!("-m tcp -p tcp --dport {redirected_port} -j REDIRECT --to-ports {target_port}");

        self.managed.remove_rule(&redirect_rule)?;

        Ok(())
    }
}
