use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Default, Deserialize, Clone)]
pub(crate) struct AppConfig {
    pub(crate) log_level: String,
    pub(crate) ttp: Ttp,
}

#[derive(Default, Deserialize, Clone)]
pub(crate) struct Ttp {
    pub(crate) gpas: Gpas,
    pub(crate) timeout: u64,
}

#[derive(Default, Deserialize, Clone, Debug)]
pub(crate) struct Gpas {
    pub(crate) base_url: String,
}

impl AppConfig {
    pub(crate) fn new() -> Result<Self, ConfigError> {
        Config::builder()
            // default config from file
            .add_source(File::with_name("app.yaml"))
            // override values from environment variables
            .add_source(Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
