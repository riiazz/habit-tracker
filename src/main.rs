mod data_updater;
mod helpers;
mod template_builder;
use std::{collections::HashMap, path::PathBuf, usize};

use chrono::{DateTime, Datelike, Duration, Utc};
use cliclack::{multiselect, select};
use google_sheets4::Sheets;
use rand::{seq::SliceRandom, thread_rng};
use serde::Deserialize;
use serde_json::Value;
use unicode_width::UnicodeWidthStr;
use yup_oauth2::{
    ServiceAccountAuthenticator,
    hyper::{self},
    hyper_rustls, read_service_account_key,
};
use yup_oauth2::{hyper::client::HttpConnector, hyper_rustls::HttpsConnector};

use crate::{
    data_updater::{bulk_update, update_today_progress},
    template_builder::generate_template_grid,
};

#[tokio::main]
async fn main() {
    let utc_now = Utc::now();
    let wib = utc_now + Duration::hours(7);
    let date_format = "%Y-%m-%d %H:%M";

    println!("{}", wib.format(date_format));
    println!();

    let app_config: AppConfig = load_app_config().await;

    println!(
        "Spreadsheet id: {}, sheet name: {}",
        app_config.spreadsheet_id, app_config.sheet_name
    );

    let auth = setup_authenticator().await;

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();

    let hub = Sheets::new(hyper::Client::builder().build(https), auth);

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

    'main_loop: loop {
        let sheet = hub
            .spreadsheets()
            .values_get(&app_config.spreadsheet_id, &app_config.sheet_name)
            .doit()
            .await;

        let mut values = sheet.unwrap_or_default().1.values.unwrap_or_default();

        let mut months: HashMap<String, usize> = values
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                row.get(0)
                    .and_then(|cell| cell.as_str())
                    .filter(|s| month_list.contains(s))
                    .map(|s| (s.to_string(), i + 1))
            })
            .collect();

        println!();

        get_today_progresses(&values, &mut months, &wib, &hub, &app_config).await;

        println!();

        let mut action_selector = select("How would you like to start?");
        action_selector = action_selector.item(1, "✅ Record today's accomplishments", "");
        action_selector = action_selector.item(2, "🔍 Browse & improve previous entries", "");
        action_selector = action_selector.item(3, "dev sandbox, insert new rows", "");

        let selected_action = action_selector.interact().unwrap();

        if selected_action == 1 {
            update_today_progress(&hub, &app_config, &wib, &mut values, &months).await;

            let is_exit = get_user_input_exit_session();

            if is_exit {
                break 'main_loop;
            }

            continue;
        } else if selected_action == 3 {
            generate_template_grid(&hub, &app_config, &wib).await;
            let is_exit = get_user_input_exit_session();
            if is_exit {
                break 'main_loop;
            }

            continue;
        }

        let (mut selected_habits, mut selected_dates, cur_month, habits, dates) =
            get_user_inputs(&values, &months);

        print_activities(
            &selected_dates,
            &selected_habits,
            &habits,
            &dates,
            &values,
            &cur_month,
            &app_config.sheet_name,
        );

        let mut is_update_selector = select("Submit selected activities?");
        is_update_selector = is_update_selector.item(true, "yes", "");
        is_update_selector = is_update_selector.item(false, "no", "");

        let is_update = is_update_selector.interact().unwrap();

        if is_update {
            bulk_update(
                &hub,
                &app_config,
                &mut values,
                &mut selected_dates,
                &mut selected_habits,
                &habits,
                &dates,
                &cur_month,
            )
            .await;
        }

        let is_exit = get_user_input_exit_session();

        if is_exit {
            break 'main_loop;
        }
    }

    print!("\nSee you tomorrow!\n");
}

fn get_user_input_exit_session() -> bool {
    let mut is_exit_selector = select("Wrap up your session? 📘");
    is_exit_selector = is_exit_selector.item(true, "Yes ✅", "");
    is_exit_selector = is_exit_selector.item(false, "No 🚫", "");
    let is_exit = is_exit_selector.interact().unwrap();
    is_exit
}

fn get_user_input_update_value() -> bool {
    let mut update_value_selector = select("Mark this habit as complete or not:");
    update_value_selector = update_value_selector.item(true, "Done ✅", "");
    update_value_selector = update_value_selector.item(false, "Skipped 🚫", "");

    let update_value = update_value_selector.interact().unwrap();
    update_value
}

fn get_user_inputs(
    values: &Vec<Vec<Value>>,
    months: &HashMap<String, usize>,
) -> (
    HashMap<String, bool>,
    HashMap<usize, bool>,
    String,
    HashMap<String, usize>,
    HashMap<usize, usize>,
) {
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

    let (cur_month, index) = months.get_key_value(&selected_month).unwrap();

    let habits = get_habits(values, index.clone());
    let dates = get_dates(values, index.clone());

    let selected_habits = get_user_input_habit(&habits);
    let selected_dates = get_user_input_date(&dates);

    (
        selected_habits,
        selected_dates,
        cur_month.to_string(),
        habits,
        dates,
    )
}

fn get_user_input_date(dates: &HashMap<usize, usize>) -> HashMap<usize, bool> {
    let mut sorted_date: Vec<(&usize, &usize)> = dates.iter().collect();
    sorted_date.sort_by_key(|(d, _)| *d);

    let mut date_selector = multiselect("Select date(s)");

    for (date, _) in sorted_date {
        date_selector = date_selector.item(date.clone(), date, "");
    }

    let selected_dates = date_selector.interact().unwrap();
    let selected_dates: HashMap<usize, bool> =
        selected_dates.into_iter().map(|h| (h, false)).collect();

    selected_dates
}

fn get_user_input_habit(habits: &HashMap<String, usize>) -> HashMap<String, bool> {
    let mut habit_selector = multiselect("Select habits");
    let mut sorted_habit: Vec<_> = habits.keys().cloned().collect();
    sorted_habit.sort();

    for habit in &sorted_habit {
        habit_selector = habit_selector.item(habit.clone(), &habit, "");
    }

    let selected_habits = habit_selector.interact().unwrap();
    let selected_habits: HashMap<String, bool> =
        selected_habits.into_iter().map(|h| (h, false)).collect();

    selected_habits
}

fn get_active_habits(values: &Vec<Vec<Value>>, index: usize) -> HashMap<String, usize> {
    let mut habits: HashMap<String, usize> = HashMap::new();

    let mut i = index;
    while i < values.len() {
        if let Some(cell) = values[i].get(0).and_then(|c| c.as_str()) {
            if cell.is_empty() {
                break;
            }

            let is_complete = values[i].get(1).and_then(|c| c.as_str());
            let is_active = values[i].get(2).and_then(|c| c.as_str());

            let is_complete = is_complete.unwrap() == "TRUE";
            let is_active = is_active.unwrap() == "TRUE";

            if !is_active || is_complete {
                i += 1;
                continue;
            }

            habits.insert(cell.to_string(), i);
        } else {
            break;
        }
        i += 1;
    }

    habits
}

fn get_habits(values: &Vec<Vec<Value>>, index: usize) -> HashMap<String, usize> {
    let mut habits: HashMap<String, usize> = HashMap::new();

    let mut i = index;
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

    habits
}

fn get_dates(values: &Vec<Vec<Value>>, index: usize) -> HashMap<usize, usize> {
    let mut dates: HashMap<usize, usize> = HashMap::new();

    let month_index = index - 1;
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

    dates
}

async fn get_today_progresses(
    values: &Vec<Vec<Value>>,
    months: &mut HashMap<String, usize>,
    wib: &DateTime<Utc>,
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
) {
    let messages = [
        "✅ You’ve completed {}! +1 EXP 🎯",
        "🔥 You nailed {}! +1 EXP",
        "🏆 Achievement unlocked: {} +1 EXP",
        "💪 Great job finishing {}! +1 EXP",
        "🌱 Progress made: {} +1 EXP",
    ];

    let current_month = wib.format("%B").to_string();
    let mut row_index = if let Some(index) = months.get(&current_month) {
        *index
    } else {
        generate_template_grid(hub, app_config, wib).await;
        months.insert(current_month.clone(), 0);
        0
    };

    let current_date = wib.day() as usize;

    let mut any_progress = false;
    let mut rng = thread_rng();
    let mut today_progress = String::from("Today's progress:\n");
    while let Some(row) = values.get(row_index) {
        if let Some(cell) = row.get(current_date) {
            let habit_name = row
                .get(0)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let cur_val = cell.as_str();

            if cur_val == Some("TRUE") {
                any_progress = true;
                let template = messages.choose(&mut rng).unwrap();
                let msg = template.replace("{}", &habit_name);
                today_progress.push_str(&msg);
                today_progress.push('\n');
            }

            row_index += 1;
        } else {
            break;
        }
    }

    if any_progress {
        println!("{}", today_progress);
    } else {
        println!("No quests completed today. The world is waiting, hero ⚔️");
    }
}

fn print_activities(
    selected_dates: &HashMap<usize, bool>,
    selected_habits: &HashMap<String, bool>,
    habits: &HashMap<String, usize>,
    dates: &HashMap<usize, usize>,
    values: &Vec<Vec<Value>>,
    cur_month: &String,
    sheet_name: &String,
) {
    let mut habit_score: HashMap<String, usize> = HashMap::new();
    println!();
    println!(
        "========================================================================================"
    );

    let width: usize = 40;

    let mut sorted_date: Vec<_> = selected_dates.keys().cloned().collect();
    sorted_date.sort();

    for date in &sorted_date {
        println!("{} {} {} activities:", date, cur_month, sheet_name);

        for (habit, _) in selected_habits {
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
    println!("Selected date stats:");

    let width: usize = 30;
    let mut total_exp = 0;
    for (habit, score) in &habit_score {
        let pad = width.saturating_sub(habit.width());
        println!("  {}{} +{} EXP", habit, " ".repeat(pad), score);
        total_exp += score;
    }

    println!(
        "\nQuest Summary: You’ve earned a total of {} EXP for the selected date(s)! ⚔️\n",
        total_exp
    );
}

async fn load_app_config() -> AppConfig {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("habit_tracker/config.toml");

    let content = tokio::fs::read_to_string(config_path)
        .await
        .expect("Failed to read config file");

    toml::from_str(&content).expect("Failed to parse config.toml")
}

async fn setup_authenticator() -> yup_oauth2::authenticator::Authenticator<
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
struct AppConfig {
    spreadsheet_id: String,
    sheet_name: String,
}
