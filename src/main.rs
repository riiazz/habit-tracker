use std::{fs, path::PathBuf, sync::Arc};

use chrono::{Duration, Utc};
use google_sheets4::Sheets;
use serde::Deserialize;
use yup_oauth2::{ServiceAccountAuthenticator, hyper, hyper_rustls, read_service_account_key};

#[tokio::main]
async fn main() {
    let utc_now = Utc::now();
    let wib = utc_now + Duration::hours(7);
    let date_format = "%Y-%m-%d %H:%M";

    println!("{}", wib.format(date_format));

    let config_path = dirs::config_dir()
        .unwrap()
        .join("habit_tracker/config.toml");

    let content = fs::read_to_string(config_path).expect("Failed to read config file");
    let app_config: AppConfig = toml::from_str(&content).expect("Failed to parse config.toml");

    println!(
        "Spreadsheet id: {}, sheet name: {}",
        app_config.spreadsheet_id, app_config.sheet_name
    );

    let creds_path: PathBuf = dirs::config_dir()
        .unwrap()
        .join("habit_tracker/credentials.json");

    let secret = read_service_account_key(&creds_path)
        .await
        .expect("Failed to read credentials.json");

    let auth = ServiceAccountAuthenticator::builder(secret)
        .build()
        .await
        .unwrap();

    //let auth = Arc::new(auth);

    println!("Successfully load credentials!");
    println!("Credential path: {}", creds_path.display());

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();

    let hub = Sheets::new(hyper::Client::builder().build(https), auth);
    let range = format!("{}!A1:AF9", app_config.sheet_name);

    let result = hub
        .spreadsheets()
        .values_get(&app_config.spreadsheet_id, &range)
        .doit()
        .await;

    match result {
        Ok((_resp, value_range)) => {
            if let Some(values) = value_range.values {
                for row in values {
                    println!("{:?}", row);
                }
            } else {
                println!("No data in range {}", range);
            }
        }
        Err(e) => {
            eprintln!("API error: {:?}", e);
        }
    }
}

#[derive(Deserialize)]
struct AppConfig {
    spreadsheet_id: String,
    sheet_name: String,
}
