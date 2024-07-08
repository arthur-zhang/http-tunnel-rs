use std::sync::Arc;
use std::time::Duration;

use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;

pub type TDNSResolver = Arc<DnsResolver>;
pub struct DnsResolver {
    _inner: TokioAsyncResolver,
}

impl DnsResolver {
    pub fn new(dns_ttl: Option<Duration>) -> Self {
        let mut opt = ResolverOpts::default();

        if dns_ttl.is_some() {
            opt.positive_max_ttl = dns_ttl;
        }
        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), opt);
        DnsResolver {
            _inner: resolver,
        }
    }
}

impl DnsResolver {
    pub async fn resolve(&self, host: &str) -> anyhow::Result<Vec<std::net::Ipv4Addr>> {
        let addrs = self._inner.ipv4_lookup(host).await?;
        Ok(addrs.iter().map(|it| it.0).collect::<Vec<_>>())
    }
}