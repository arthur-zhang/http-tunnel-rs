use std::sync::Arc;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;

pub type TDNSResolver = Arc<DnsResolver>;
pub struct DnsResolver {
    _inner: TokioAsyncResolver,
}

impl DnsResolver {
    pub fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
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