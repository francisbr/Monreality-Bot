use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppSettings {
    pub discord_token: String,
    pub guild_id: u64,
    pub mute_duration: u64,
    pub redis_url: String,
}

impl AppSettings {
    pub fn new() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name("config.toml"))
            .add_source(Environment::default())
            .build()?
            .try_deserialize()
    }
}
