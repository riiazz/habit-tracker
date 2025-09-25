use std::{collections::HashMap, fs, path::PathBuf, usize};

use chrono::{Duration, Utc};
use cliclack::{multiselect, select};
use google_sheets4::Sheets;
use serde::Deserialize;
use unicode_width::UnicodeWidthStr;
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

    println!("Credential path: {}", creds_path.display());
    println!();

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();

    let hub = Sheets::new(hyper::Client::builder().build(https), auth);
    //let range = format!("{}!A1:AF9", app_config.sheet_name);

    let sheet = hub
        .spreadsheets()
        .values_get(&app_config.spreadsheet_id, &app_config.sheet_name)
        .doit()
        .await;

    let values = sheet.unwrap_or_default().1.values.unwrap_or_default();

    let month_list = vec![
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    let months: HashMap<String, usize> = values
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            row.get(0)
                .and_then(|cell| cell.as_str())
                .filter(|s| month_list.contains(s))
                .map(|s| (s.to_string(), i + 1))
        })
        .collect();

    let mut month_selector = select("Select month");

    let mut sorted_month_by_index: Vec<(&String, &usize)> = months.iter().collect();
    sorted_month_by_index.sort_by_key(|(_, i)| *i);

    for (month, _) in sorted_month_by_index {
        month_selector = month_selector.item(month.clone(), month, "");
    }

    let selected_month = month_selector.interact().unwrap();

    println!(
        "Selected month {} with index {}",
        &selected_month,
        &months.get(&selected_month).unwrap()
    );

    let mut habits: HashMap<String, usize> = HashMap::new();
    let mut i = months.get(&selected_month).unwrap().clone();
    while i < values.len() {
        if let Some(cell) = values[i].get(0).and_then(|c| c.as_str()) {
            if cell.is_empty() {
                break;
            }
            habits.insert(cell.to_string(), i);
        } else {
            break;
        }
        i += 1;
    }

    let mut habit_selector = multiselect("Select habits");

    for (habit, _) in &habits {
        habit_selector = habit_selector.item(habit.clone(), &habit, "");
    }

    let selected_habits = habit_selector.interact().unwrap();

    let mut dates: HashMap<usize, usize> = HashMap::new();
    let (cur_month, index) = months.get_key_value(&selected_month).unwrap();
    let month_index = index.clone() - 1;
    let mut i = 1;
    while i < values[month_index].len() {
        if let Some(cell) = values[month_index].get(i).and_then(|c| c.as_str()) {
            if cell.is_empty() || !cell.parse::<usize>().is_ok() {
                break;
            }
            dates.insert(cell.parse::<usize>().unwrap(), i);
        } else {
            break;
        }
        i += 1;
    }

    //for (date, _) in &dates {
    //    println!("{} date: {}", &cur_month, &date);
    //}

    let mut sorted_date: Vec<(&usize, &usize)> = dates.iter().collect();
    sorted_date.sort_by_key(|(d, _)| *d);

    let mut date_selector = multiselect("Select date(s)");

    for (date, _) in sorted_date {
        date_selector = date_selector.item(date.clone(), date, "");
    }

    let selected_dates = date_selector.interact().unwrap();

    let mut habit_score: HashMap<String, usize> = HashMap::new();
    println!();
    println!(
        "========================================================================================"
    );

    let width: usize = 40;
    for date in &selected_dates {
        println!(
            "{} {} {} activities:",
            date, cur_month, &app_config.sheet_name
        );

        for habit in &selected_habits {
            let date_index = dates.get(date).unwrap();
            let habit_index = habits.get(habit).unwrap();
            let is_done = values[*habit_index].get(*date_index).unwrap() == "TRUE";
            let message = if is_done { "✅✅✅" } else { "❌❌❌" };

            let pad = width.saturating_sub(habit.width());
            println!("  {}{}{}", habit, " ".repeat(pad), message);

            if is_done {
                *habit_score.entry(habit.to_string()).or_insert(0) += 1;
            }
        }
        println!();
    }

    println!();
    println!("Total streak across selected dates:");

    let width: usize = 30;
    for (habit, score) in &habit_score {
        let pad = width.saturating_sub(habit.width());
        println!("  {}{}{} streaks", habit, " ".repeat(pad), score);
    }
}

#[derive(Deserialize)]
struct AppConfig {
    spreadsheet_id: String,
    sheet_name: String,
}
