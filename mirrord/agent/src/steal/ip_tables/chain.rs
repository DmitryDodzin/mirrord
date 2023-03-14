use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};

use crate::{
    error::{AgentError, Result},
    steal::ip_tables::IPTables,
};

#[derive(Debug)]
pub struct IPTableChain<IPT> {
    inner: Arc<IPT>,
    chain: String,
    chain_size: AtomicI32,
}

impl<IPT> IPTableChain<IPT>
where
    IPT: IPTables,
{
    pub fn create(inner: Arc<IPT>, chain: String) -> Result<Self> {
        inner.create_chain(&chain)?;

        // Start with 1 because the chain will allways have atleast `-A <chain name>` as a rule
        let chain_size = AtomicI32::from(1);

        Ok(IPTableChain {
            inner,
            chain,
            chain_size,
        })
    }

    pub fn load(inner: Arc<IPT>, chain: String) -> Result<Self> {
        let rules = inner.list_rules(&chain)?.len();

        if rules == 0 {
            return Err(AgentError::IPTablesError(format!(
                "Unable to load chain {chain} because no rules exist"
            )));
        }

        let chain_size = AtomicI32::from((rules - 1) as i32);

        Ok(IPTableChain {
            inner,
            chain,
            chain_size,
        })
    }

    pub fn get_chain_name(&self) -> &str {
        &self.chain
    }

    pub fn add_rule(&self, rule: &str) -> Result<i32> {
        self.inner
            .insert_rule(
                &self.chain,
                rule,
                self.chain_size.fetch_add(1, Ordering::Relaxed),
            )
            .map(|_| self.chain_size.load(Ordering::Relaxed))
            .map_err(|err| {
                self.chain_size.fetch_sub(1, Ordering::Relaxed);
                err
            })
    }

    pub fn remove_rule(&self, rule: &str) -> Result<()> {
        self.inner.remove_rule(&self.chain, rule)?;

        self.chain_size.fetch_sub(1, Ordering::Relaxed);

        Ok(())
    }
}
