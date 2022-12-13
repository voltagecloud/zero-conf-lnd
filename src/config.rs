use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub(crate) struct Config {
    pub lnd: LndConfig,
    pub channel_acceptance: Vec<ChannelAcceptanceParams>,
}
#[derive(Deserialize, Clone)]
pub(crate) struct LndConfig {
    #[serde(default = "default_lnd_macaroon_location")]
    pub macaroon_location: String,

    #[serde(default = "default_lnd_cert_location")]
    pub cert_location: String,

    #[serde(default = "default_lnd_host")]
    pub host: String,

    #[serde(default = "default_lnd_port")]
    pub port: u16,
}

#[derive(Deserialize, Clone)]
pub(crate) struct ChannelAcceptanceParams {
    #[serde(default = "default_empty_string")]
    pub pubkey: String,

    #[serde(default = "default_confs")]
    pub confs: u8,
}

fn default_lnd_macaroon_location() -> String {
    String::from("~/.lnd/data/chain/bitcoin/mainnet/admin.macaroon")
}

fn default_lnd_cert_location() -> String {
    String::from("~/.lnd/tls.cert")
}

fn default_lnd_host() -> String {
    String::from("127.0.0.1")
}

fn default_lnd_port() -> u16 {
    9735
}

fn default_empty_string() -> String {
    String::from("")
}

fn default_confs() -> u8 {
    0
}

impl Config {
    pub fn new() -> Self {
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| String::from("config.yml"));
        match std::fs::File::open(config_path) {
            Ok(config_file) => {
                let config: Config =
                    serde_yaml::from_reader(config_file).expect("yaml formating error");
                config
            }
            Err(_) => {
                tracing::debug!("no config file found, loading defaults...");
                Config {
                    lnd: LndConfig {
                        macaroon_location: default_lnd_macaroon_location(),
                        cert_location: default_lnd_cert_location(),
                        host: default_lnd_host(),
                        port: default_lnd_port(),
                    },
                    channel_acceptance: vec![],
                }
            }
        }
    }
}
