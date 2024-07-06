use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::{debug, error, info};
use rand::{random, Rng, thread_rng};
use tokio::io;
use tokio::sync::RwLock;
type CachedSocketAddrs = (Vec<SocketAddr>, u128);

#[derive(Clone)]
pub struct SimpleCachingDnsResolver {
    // mostly reads, occasional writes
    cache: Arc<RwLock<HashMap<String, CachedSocketAddrs>>>,
    ttl: Duration,
    start_time: Instant,
}

impl SimpleCachingDnsResolver{

    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            start_time: Instant::now(),
        }
    }
    async fn try_find(&mut self, target: &str) -> Option<SocketAddr> {
        let map = self.cache.read().await;

        let addr = match map.get(target) {
            None => None,
            Some((cached, expiration)) => {
                // expiration with jitter to avoid expiration "waves"
                let expiration_jitter = *expiration + rand::thread_rng().gen_range(0..5_000);
                if Instant::now().duration_since(self.start_time).as_millis() < expiration_jitter {
                    Some(self.pick(cached))
                } else {
                    None
                }
            }
        };

        addr
    }
    fn pick(&self, addrs: &[SocketAddr]) -> SocketAddr {
        addrs[random::<usize>() % addrs.len()]
    }
    async fn resolve_and_cache(&mut self, target: &str) -> io::Result<SocketAddr> {
        let resolved = SimpleCachingDnsResolver::resolve_inner(target).await?;

        let mut map = self.cache.write().await;
        map.insert(
            target.to_string(),
            (
                resolved.clone(),
                Instant::now().duration_since(self.start_time).as_millis() + self.ttl.as_millis(),
            ),
        );

        Ok(self.pick(&resolved))
    }
    async fn resolve_inner(target: &str) -> io::Result<Vec<SocketAddr>> {
        debug!("Resolving DNS {}", target);
        let resolved: Vec<SocketAddr> = tokio::net::lookup_host(target).await?.collect();
        debug!("Resolved DNS {} to {:?}", target, resolved);

        if resolved.is_empty() {
            error!("Cannot resolve DNS {}", target);
            return Err(Error::from(ErrorKind::AddrNotAvailable));
        }

        Ok(resolved)
    }
    pub async fn resolve(&mut self, target: &str) -> io::Result<SocketAddr> {
        match self.try_find(target).await {
            Some(a) => Ok(a),
            _ => Ok(self.resolve_and_cache(target).await?),
        }
    }
}