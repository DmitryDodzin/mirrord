use std::sync::Arc;

use async_trait::async_trait;
use mirrord_protocol::Port;

use crate::{
    error::Result,
    steal::ip_tables::{chain::IPTableChain, IPTables, Redirect, IPTABLE_MANGLE},
};

pub(crate) struct MangleRedirect<IPT: IPTables, T> {
    pub(crate) managed: IPTableChain<IPT>,
    inner: Box<T>,
}

impl<IPT, T> MangleRedirect<IPT, T>
where
    IPT: IPTables,
    T: Redirect,
{
    const ENTRYPOINT: &'static str = "OUTPUT";

    pub fn create(ipt: Arc<IPT>, inner: Box<T>) -> Result<Self> {
        let managed = IPTableChain::create(ipt, IPTABLE_MANGLE.to_string())?;

        Ok(MangleRedirect { managed, inner })
    }

    pub fn load(ipt: Arc<IPT>, inner: Box<T>) -> Result<Self> {
        let managed = IPTableChain::load(ipt, IPTABLE_MANGLE.to_string())?;

        Ok(MangleRedirect { managed, inner })
    }
}

#[async_trait]
impl<IPT, T> Redirect for MangleRedirect<IPT, T>
where
    IPT: IPTables + Send + Sync,
    T: Redirect + Send + Sync,
{
    #[tracing::instrument(level = "trace", skip(self), ret)]
    async fn mount_entrypoint(&self) -> Result<()> {
        self.inner.mount_entrypoint().await?;

        self.managed.inner().insert_rule(
            Self::ENTRYPOINT,
            &format!("-j {}", self.managed.chain_name()),
            1,
        )?;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self), ret)]
    async fn unmount_entrypoint(&self) -> Result<()> {
        self.inner.unmount_entrypoint().await?;

        self.managed.inner().remove_rule(
            Self::ENTRYPOINT,
            &format!("-j {}", self.managed.chain_name()),
        )?;

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self), ret)]
    async fn add_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        self.inner
            .add_redirect(redirected_port, target_port)
            .await?;

        let redirect_rule =
            format!("-d 127.0.0.1/32 -p tcp -m mark --mark 0x539/0xfff --dport {redirected_port} -j REDIRECT --to-ports {target_port}");

        if let Err(error) = self.managed.add_rule(&redirect_rule) {
            let dmesg = tokio::process::Command::new("dmesg").output().await;
            tracing::error!(?error, ?dmesg, "error adding mangle redirect");
        }

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self), ret)]
    async fn remove_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        self.inner
            .remove_redirect(redirected_port, target_port)
            .await?;

        let redirect_rule =
            format!("-d 127.0.0.1/32 -p tcp -m mark --mark 0x539/0xfff --dport {redirected_port} -j REDIRECT --to-ports {target_port}");

        if let Err(error) = self.managed.remove_rule(&redirect_rule) {
            let dmesg = tokio::process::Command::new("dmesg").output().await;
            tracing::error!(?error, ?dmesg, "error removing mangle redirect");
        }

        Ok(())
    }
}
