use std::time::Duration;

use anyhow::bail;
use clap::Parser;
use serde::Deserialize;

pub const NO_TIMEOUT: Duration = Duration::from_secs(300);

#[derive( Deserialize, Clone, Debug)]
pub struct RelayPolicy {
    #[serde(with = "humantime_serde")]
    pub idle_timeout: Duration,
}

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

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            client_connection: ClientConnectionConfig {
                // initiation_timeout: NO_TIMEOUT,
                // relay_policy: RelayPolicy {
                //     idle_timeout: NO_TIMEOUT,
                // },
            },
            target_connection: TargetConnectionConfig {
                dns_cache_ttl: NO_TIMEOUT,
                // connect_timeout: NO_TIMEOUT,
                // relay_policy: RelayPolicy {
                //     idle_timeout: NO_TIMEOUT,
                // },
            },
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct TunnelConfig {
    pub client_connection: ClientConnectionConfig,
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


#[derive(Deserialize, Debug)]
pub struct HttpConfig {
    pub listen_port: u16,
}
#[derive(Deserialize, Debug)]
pub struct HttpsConfig {
    pub listen_port: u16,
}
#[derive(Deserialize, Debug, Clone)]
pub struct TcpConfig {
    pub listen_port: u16,
    pub remote_addr: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClientConnectionConfig {
    // #[serde(with = "humantime_serde")]
    // pub initiation_timeout: Duration,
    // #[serde(with = "humantime_serde")]
    // pub idle_timeout: Duration,
    // #[serde(flatten)]
    // pub relay_policy: RelayPolicy,
}

impl Default for ClientConnectionConfig {
    fn default() -> Self {
        Self {
            // initiation_timeout: Duration::from_secs(10),
            // relay_policy: RelayPolicy {
            //     idle_timeout: Duration::from_secs(30),
            // },
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TargetConnectionConfig {
    #[serde(with = "humantime_serde")]
    pub dns_cache_ttl: Duration,
    // #[serde(with = "humantime_serde")]
    // pub connect_timeout: Duration,
    // #[serde(flatten)]
    // pub relay_policy: RelayPolicy,

}

// #[derive(Builder, Deserialize, Clone, Debug)]
// pub struct RelayPolicy {
//     #[serde(with = "humantime_serde")]
//     pub idle_timeout: Duration,
// }


impl Default for TargetConnectionConfig {
    fn default() -> Self {
        Self {
            dns_cache_ttl: Duration::from_secs(60),
            // connect_timeout: Duration::from_secs(10),
            // relay_policy: RelayPolicy {
            //     idle_timeout: Duration::from_secs(30),
            // },
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