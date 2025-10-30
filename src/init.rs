use std::path::PathBuf;

use serde::Deserialize;
use yup_oauth2::{ServiceAccountAuthenticator, read_service_account_key};

pub async fn load_app_config() -> AppConfig {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("habit_tracker/config.toml");

    let content = tokio::fs::read_to_string(config_path)
        .await
        .expect("Failed to read config file");

    toml::from_str(&content).expect("Failed to parse config.toml")
}

pub async fn setup_authenticator() -> yup_oauth2::authenticator::Authenticator<
    yup_oauth2::hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>,
> {
    let creds_path: PathBuf = dirs::config_dir()
        .unwrap()
        .join("habit_tracker/credentials.json");

    println!("Credential path: {}", creds_path.display());
    println!();

    let secret = read_service_account_key(&creds_path)
        .await
        .expect("Failed to read credentials.json");

    ServiceAccountAuthenticator::builder(secret)
        .build()
        .await
        .expect("Failed to build authenticator")
}

#[derive(Deserialize)]
pub struct AppConfig {
    pub spreadsheet_id: String,
    pub sheet_name: String,
}
