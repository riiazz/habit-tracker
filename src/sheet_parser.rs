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
        if let Some(cell) = values[i]
            .get(config_table::Column::HabitName.as_usize_zero_based_index())
            .and_then(|c| c.as_str())
        {
            if cell.is_empty() {
                break;
            }

            let is_complete = values[i]
                .get(config_table::Column::IsComplete.as_usize_zero_based_index())
                .and_then(|c| c.as_str());
            let is_active = values[i]
                .get(config_table::Column::IsActive.as_usize_zero_based_index())
                .and_then(|c| c.as_str());

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

pub async fn get_sheet_id(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    sheet_name: &String,
) -> i32 {
    let (_, spreadsheet) = hub
        .spreadsheets()
        .get(&app_config.spreadsheet_id)
        .doit()
        .await
        .unwrap();

    let sheet_id = spreadsheet
        .sheets
        .as_ref()
        .and_then(|sheets| {
            sheets
                .iter()
                .filter_map(|sheet| sheet.properties.as_ref())
                .find(|props| props.title.as_deref() == Some(sheet_name))
                .and_then(|props| props.sheet_id)
        })
        .unwrap_or_else(|| panic!("Sheet id for {} not found!", sheet_name));

    sheet_id
}

pub fn print_current_month_total_progress(values: &Vec<Vec<Value>>) {
    let habits = get_habits(values, 0);
    let dates = get_dates(values, 1);
    let mut habit_score: HashMap<String, usize> = HashMap::new();

    for (habit, row) in habits {
        for (_, col) in &dates {
            if values[row][*col] == "TRUE" {
                *habit_score.entry(habit.to_string()).or_insert(0) += 1;
            }
        }
    }

    let openings = [
        "Hero’s Monthly Report:",
        "Your Adventure Log for this month:",
        "Guild Ledger — Monthly Summary:",
        "The Oracle reveals your progress:",
        "Record of your deeds this month:",
        "Your expedition results:",
        "Monthly EXP Tally:",
        "Brave adventurer, here are your gains:",
        "Warrior, your training for this month is logged:",
        "Chronicles of the month:",
    ];

    let bodies = [
        "You have gained the following EXP:",
        "These are the fruits of your discipline:",
        "Your actions have yielded:",
        "Your quests granted you:",
        "Experience accumulated from your habits:",
        "Behold your earned experience:",
    ];

    let total_message = [
        "Your total experience this month amounts to {total_exp} EXP.",
        "You have amassed a total of {total_exp} EXP from all habits.",
        "All quests combined, your EXP reaches {total_exp}.",
        "The guild tallies your monthly total: {total_exp} EXP.",
        "Your combined training grants you {total_exp} EXP.",
        "The Chronicle Keeper records a total of {total_exp} EXP.",
        "The Oracle reveals your essence: {total_exp} EXP earned.",
        "System report: Total accumulated EXP = {total_exp}.",
        "Total EXP acquired this month: {total_exp}.",
        "Your journey’s monthly sum stands at {total_exp} EXP.",
        "By all your deeds, you have secured {total_exp} EXP.",
        "The month concludes with {total_exp} EXP earned.",
        "Computation complete. Total EXP: {total_exp}.",
        "Your saga grows with this month's total of {total_exp} EXP.",
        "Final tally: {total_exp} EXP gained.",
        "Hero, your power this month totals {total_exp} EXP.",
        "Record update: Total monthly EXP = {total_exp}.",
        "Your efforts yield a combined {total_exp} EXP.",
        "These results grant you {total_exp} EXP in total.",
        "The grand total of your habit EXP is {total_exp}.",
    ];

    let closings = [
        "Press onward, hero.",
        "Your journey continues.",
        "May next month be even stronger.",
        "The guild is proud.",
        "Your legend grows.",
        "Stay steadfast, warrior.",
        "Another month awaits.",
        "Your path becomes clearer.",
        "Victory is built day by day.",
        "Return soon with new triumphs.",
    ];

    println!(
        "\n{}\n{}\n",
        random_element(&openings),
        random_element(&bodies)
    );

    let mut sorted_habit: Vec<String> = habit_score.keys().cloned().collect();
    sorted_habit.sort();

    let mut total_exp = 0;
    for habit in sorted_habit {
        let score = habit_score.get(&habit).unwrap();
        println!("{} : {} EXP", habit, score);
        total_exp = total_exp + score;
    }

    let total_message = random_element(&total_message);
    let total_message = total_message.replace("{total_exp}", &total_exp.to_string());

    println!("\n{}\n{}\n", total_message, random_element(&closings));
}

fn random_element<'a>(list: &'a [&str]) -> &'a str {
    list.choose(&mut rand::thread_rng()).unwrap()
}

pub mod config_table {
    pub const START_ROW_INDEX: usize = 1;

    #[derive(Debug, Clone, Copy)]
    pub enum Column {
        HabitName = 1,
        IsComplete = 2,
        IsActive = 3,
    }

    impl Column {
        pub fn as_usize(self) -> usize {
            self as usize
        }

        pub fn as_usize_zero_based_index(self) -> usize {
            self as usize - 1
        }
    }
}
