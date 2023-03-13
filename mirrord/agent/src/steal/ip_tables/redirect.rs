use std::sync::LazyLock;

use mirrord_protocol::Port;
use rand::distributions::{Alphanumeric, DistString};

use crate::{
    error::Result,
    steal::ip_tables::{chain::IPTableChain, IPTables},
};

static IPTABLE_PREROUTING_ENV: &str = "MIRRORD_IPTABLE_PREROUTING_NAME";
static IPTABLE_PREROUTING: LazyLock<String> = LazyLock::new(|| {
    std::env::var(IPTABLE_PREROUTING_ENV).unwrap_or_else(|_| {
        format!(
            "MIRRORD_INPUT_{}",
            Alphanumeric.sample_string(&mut rand::thread_rng(), 5)
        )
    })
});

pub trait Redirect {
    /// Create port redirection
    fn add_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()>;
    /// Remove port redirection
    fn remove_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()>;
}

pub struct PreroutingRedirect<'ipt, IPT> {
    managed: IPTableChain<'ipt, IPT>,
}

impl<'ipt, IPT> PreroutingRedirect<'ipt, IPT>
where
    IPT: IPTables,
{
    pub fn create(ipt: &'ipt IPT) -> Self {
        let managed = IPTableChain::new(ipt, "nat", &IPTABLE_PREROUTING);

        PreroutingRedirect { managed }
    }
}

impl<IPT> Redirect for PreroutingRedirect<'_, IPT>
where
    IPT: IPTables,
{
    fn add_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        let redirect_rule =
            format!("-m tcp -p tcp --dport {redirected_port} -j REDIRECT --to-ports {target_port}");

        self.managed.add_rule(&redirect_rule)?;

        Ok(())
    }

    fn remove_redirect(&self, redirected_port: Port, target_port: Port) -> Result<()> {
        let redirect_rule =
            format!("-m tcp -p tcp --dport {redirected_port} -j REDIRECT --to-ports {target_port}");

        self.managed.remove_rule(&redirect_rule)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::*;

    use super::*;
    use crate::steal::ip_tables::MockIPTables;

    #[test]
    fn add_redirect() {
        let mut mock = MockIPTables::new();

        mock.expect_insert_rule()
            .with(
                eq(*IPTABLE_PREROUTING),
                eq("-m tcp -p tcp --dport 69 -j REDIRECT --to-ports 420"),
                eq(1),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        let prerouting = PreroutingRedirect::create(&mock);

        assert!(prerouting.add_redirect(69, 420).is_ok());
    }
}