use std::collections::HashMap;

use chrono::{DateTime, Datelike, Utc};
use google_sheets4::Sheets;
use rand::{seq::SliceRandom, thread_rng};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;
use yup_oauth2::{hyper::client::HttpConnector, hyper_rustls::HttpsConnector};

use crate::{AppConfig, init::valid_months, template_builder::generate_template_grid};

pub fn get_active_habits(values: &Vec<Vec<Value>>, index: usize) -> HashMap<String, usize> {
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

pub fn get_habits(values: &Vec<Vec<Value>>, index: usize) -> HashMap<String, usize> {
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

pub fn get_dates(values: &Vec<Vec<Value>>, index: usize) -> HashMap<usize, usize> {
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

pub async fn get_today_progresses(
    values: &mut Vec<Vec<Value>>,
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
        println!(
            "⚡ '{}' missing from database. Initiating reconstruction protocol... 🚧",
            current_month
        );

        let (new_value, _) = generate_template_grid(hub, app_config, wib).await;
        *values = new_value;
        *months = valid_months(values);
        *months
            .get(&current_month)
            .expect("Failed generating new month grid")
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

pub fn print_activities(
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
