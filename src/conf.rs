use std::time::Duration;

use anyhow::bail;
use clap::Parser;
use serde::Deserialize;


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(short, long)]
    config: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub http: Option<HttpConfig>,
    pub https: Option<HttpsConfig>,
    #[serde(default)]
    pub tcp: Vec<TcpConfig>,
    #[serde(flatten, default)]
    pub tunnel_config: TunnelConfig,

}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct TunnelConfig {
    pub target_connection: TargetConnectionConfig,
}


impl Config {
    pub fn from_cmd_line() -> anyhow::Result<Config> {
        let cli = Cli::parse();
        if let Some(config_path) = cli.config {
            let config_str = std::fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&config_str)?;
            return Ok(config);
        }
        bail!("Config file not found")
    }
}


#[derive(Deserialize, Debug, Clone)]
pub struct HttpConfig {
    pub listen_port: u16,
}
#[derive(Deserialize, Debug, Clone)]
pub struct HttpsConfig {
    pub listen_port: u16,
}
#[derive(Deserialize, Debug, Clone)]
pub struct TcpConfig {
    pub listen_port: u16,
    pub remote_addr: String,
}


#[derive(Deserialize, Debug, Clone)]
pub struct TargetConnectionConfig {
    #[serde(with = "humantime_serde")]
    pub dns_cache_ttl: Option<Duration>,
    #[serde(with = "humantime_serde")]
    pub connect_timeout: Duration,
}


impl Default for TargetConnectionConfig {
    fn default() -> Self {
        Self {
            dns_cache_ttl: None,
            connect_timeout: Duration::from_secs(10),
        }
    }
}

#[cfg(test)]
mod tests {
    use log::info;

    use crate::conf::Config;

    #[test]
    fn test_conf_parse() {
        let conf = r#"
[http]
listen_port = 8081

[[tcp]]
listen_port = 8082
remote_addr = "192.168.31.197:22"

[[tcp]]
listen_port = 8083
remote_addr = "192.168.31.197:80"

[client_connection]
initiation_timeout_seconds = 10
relay_timeout_seconds = 30

[remote_connection]
dns_cache_ttl_seconds = 60
connect_timeout_seconds = 10
idle_timeout_seconds = 30
"#;

        let config: Config = toml::from_str(conf).unwrap();

        info!("{:?}", config)
    }
}