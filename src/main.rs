use std::path::PathBuf;

use chrono::{Duration, Utc};
use yup_oauth2::{ServiceAccountAuthenticator, read_service_account_key};

#[tokio::main]
async fn main() {
    let utc_now = Utc::now();
    let wib = utc_now + Duration::hours(7);
    let date_format = "%Y-%m-%d %H:%M";

    println!("{}", wib.format(date_format));

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

    println!("Successfully load credentials!");
    println!("Credential path: {}", creds_path.display());
}
