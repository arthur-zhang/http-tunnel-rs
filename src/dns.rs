use std::sync::Arc;
use std::time::Duration;

use hickory_resolver::system_conf::read_system_conf;
use hickory_resolver::TokioAsyncResolver;

pub type TDNSResolver = Arc<DnsResolver>;
pub struct DnsResolver {
    _inner: TokioAsyncResolver,
}

impl DnsResolver {
    pub fn new(dns_ttl: Option<Duration>) -> anyhow::Result<Self> {
        let (sys_config, mut sys_options) = read_system_conf().map_err(|e| anyhow::anyhow!(e))?;
        if dns_ttl.is_some() {
            sys_options.positive_max_ttl = dns_ttl;
        }

        let resolver = TokioAsyncResolver::tokio(sys_config, sys_options);

        Ok(DnsResolver { _inner: resolver })
    }
}

impl DnsResolver {
    pub async fn resolve(&self, host: &str) -> anyhow::Result<Vec<std::net::Ipv4Addr>> {
        let addrs = self._inner.ipv4_lookup(host).await?;
        Ok(addrs.iter().map(|it| it.0).collect::<Vec<_>>())
    }
}