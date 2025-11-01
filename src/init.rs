use std::{collections::HashMap, path::PathBuf, str::FromStr};

use chrono::{DateTime, Datelike, Utc};
use google_sheets4::Sheets;
use serde::Deserialize;
use serde_json::Value;
use time::Month;
use yup_oauth2::{
    ServiceAccountAuthenticator, hyper::client::HttpConnector, hyper_rustls::HttpsConnector,
    read_service_account_key,
};

use crate::template_builder::{auto_resize_dimension, generate_sheet, generate_template_grid};

pub async fn load_app_config(date_time: DateTime<Utc>) -> AppConfig {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("habit_tracker/config.toml");

    let content = tokio::fs::read_to_string(config_path)
        .await
        .expect("Failed to read config file");

    let mut app_config: AppConfig = toml::from_str(&content).expect("Failed to parse config.toml");
    app_config.sheet_name = date_time.year().to_string();

    app_config
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

pub async fn ensure_sheet_ready(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    wib: &DateTime<Utc>,
) -> Vec<Vec<Value>> {
    let mut sheet = hub
        .spreadsheets()
        .values_get(&app_config.spreadsheet_id, &app_config.sheet_name)
        .doit()
        .await;

    let values = match sheet {
        Ok((_, value_range)) => value_range,
        Err(_) => {
            println!(
                "⚡ Sheet '{}' missing from database. Initiating reconstruction protocol... 🚧",
                app_config.sheet_name
            );

            generate_sheet(&hub, &app_config).await;
            let (_, sheet_id) = generate_template_grid(&hub, &app_config, &wib).await;
            auto_resize_dimension(&hub, &app_config, sheet_id).await;

            sheet = hub
                .spreadsheets()
                .values_get(&app_config.spreadsheet_id, &app_config.sheet_name)
                .doit()
                .await;

            match sheet {
                Ok((_, value_range)) => {
                    println!(
                        "✅ Sheet '{}' created successfully! You’re all set to continue. 🎉",
                        app_config.sheet_name
                    );
                    value_range
                }
                Err(_) => {
                    panic!("Creating new sheet failed, make sure you have internet connection")
                }
            }
        }
    };

    values.values.unwrap_or_default()
}

pub fn valid_months(values: &Vec<Vec<Value>>) -> HashMap<String, usize> {
    values
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            row.get(0)
                .and_then(|cell| cell.as_str())
                .and_then(|s| Month::from_str(s).ok())
                .map(|m| (m.to_string(), i + 1))
        })
        .collect()
}
