use std::collections::HashMap;

use chrono::{DateTime, Datelike, Utc};
use cliclack::{multiselect, select};
use google_sheets4::{
    Sheets,
    api::{BatchUpdateValuesRequest, ValueRange},
};
use serde_json::Value;
use yup_oauth2::{hyper::client::HttpConnector, hyper_rustls::HttpsConnector};

use crate::{
    AppConfig, get_habits, get_user_input_habit, get_user_input_update_value, print_activities,
};

pub async fn update_today_progress(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    wib: &DateTime<Utc>,
    values: &mut Vec<Vec<Value>>,
    months: &HashMap<String, usize>,
) {
    let current_month = wib.format("%B").to_string();
    let month_index = months.get(&current_month).unwrap();
    let habits = get_habits(&values, month_index.clone());
    let mut selected_habits = get_user_input_habit(&habits);

    let update_value = get_user_input_update_value();

    selected_habits.iter_mut().for_each(|(_, v)| *v = true);

    update_activities(
        &HashMap::from([(wib.day() as usize, true)]),
        &selected_habits,
        &habits,
        &HashMap::from([(wib.day() as usize, wib.day() as usize)]),
        values,
        &current_month,
        &app_config,
        &hub,
        update_value,
    )
    .await;
}

pub async fn bulk_update(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    values: &mut Vec<Vec<Value>>,
    selected_dates: &mut HashMap<usize, bool>,
    selected_habits: &mut HashMap<String, bool>,
    habits: &HashMap<String, usize>,
    dates: &HashMap<usize, usize>,
    cur_month: &String,
) {
    let mut is_update_all_selected_selector = select("Mark all selected as done/undone? 🎯");
    is_update_all_selected_selector = is_update_all_selected_selector.item(true, "yes", "");
    is_update_all_selected_selector = is_update_all_selected_selector.item(false, "no", "");

    let is_submit_all = is_update_all_selected_selector.interact().unwrap();

    if !is_submit_all {
        let mut habit_selector = multiselect("Select habits");

        let mut sorted_habit: Vec<_> = selected_habits.keys().cloned().collect();
        sorted_habit.sort();
        for habit in &sorted_habit {
            habit_selector = habit_selector.item(habit.clone(), &habit, "");
        }

        let keep_habit = habit_selector.interact().unwrap();

        let mut date_selector = multiselect("Select dates");

        let mut sorted_date: Vec<_> = selected_dates.keys().cloned().collect();
        sorted_date.sort();
        for date in &sorted_date {
            date_selector = date_selector.item(date.clone(), &date, "");
        }

        let keep_date = date_selector.interact().unwrap();

        for habit in &keep_habit {
            if let Some(value) = selected_habits.get_mut(habit) {
                *value = true;
            }
        }

        for date in &keep_date {
            if let Some(value) = selected_dates.get_mut(date) {
                *value = true;
            }
        }
    } else {
        selected_habits.iter_mut().for_each(|(_, v)| *v = true);
        selected_dates.iter_mut().for_each(|(_, v)| *v = true);
    }

    let update_value = get_user_input_update_value();

    update_activities(
        &selected_dates,
        &selected_habits,
        &habits,
        &dates,
        values,
        &cur_month,
        &app_config,
        &hub,
        update_value,
    )
    .await;
}

pub async fn update_activities(
    selected_dates: &HashMap<usize, bool>,
    selected_habits: &HashMap<String, bool>,
    habits: &HashMap<String, usize>,
    dates: &HashMap<usize, usize>,
    values: &mut Vec<Vec<Value>>,
    cur_month: &String,
    app_config: &AppConfig,
    hub: &Sheets<
        yup_oauth2::hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>,
    >,
    update_value: bool,
) {
    let mut updated_cell: Vec<ValueRange> = Vec::new();
    let update_value = if update_value { "TRUE" } else { "FALSE" };

    for (habit, is_update) in selected_habits {
        if !is_update {
            continue;
        }
        let habit = habits.get(habit).unwrap();

        for (date, is_update) in selected_dates {
            if !is_update {
                continue;
            }
            let date = dates.get(date).unwrap();

            let cell_address = cell_address(*habit + 1, *date + 1);
            set_data(
                &mut updated_cell,
                update_value.to_string(),
                cell_address,
                &app_config.sheet_name,
            );

            values[*habit][*date] = Value::String(update_value.to_string());
        }
    }

    let batch = BatchUpdateValuesRequest {
        value_input_option: Some("USER_ENTERED".to_string()),
        data: Some(updated_cell),
        ..Default::default()
    };

    let result = hub
        .spreadsheets()
        .values_batch_update(batch, &app_config.spreadsheet_id)
        .doit()
        .await;

    match result {
        Ok((_, response)) => {
            println!(
                "{} cells updated",
                response.total_updated_cells.unwrap_or(0)
            );

            print_activities(
                &selected_dates,
                &selected_habits,
                &habits,
                &dates,
                &values,
                cur_month,
                &app_config.sheet_name,
            );
        }
        Err(err) => {
            eprint!("Update failed: {:?}", err);
        }
    }
}

pub fn set_data(
    value_range: &mut Vec<ValueRange>,
    cell_value: String,
    cell_index: String,
    sheet_name: &String,
) {
    let value: Value = Value::String(cell_value.clone());
    value_range.push(ValueRange {
        range: Some(format!("{}!{}", sheet_name, cell_index)),
        values: Some(vec![vec![value]]),
        ..Default::default()
    });
}

fn column_to_letter(mut col: usize) -> String {
    let mut result = String::new();
    while col > 0 {
        let rem = (col - 1) % 26;
        result.insert(0, (b'A' + rem as u8) as char);
        col = (col - 1) / 26;
    }
    result
}

pub fn cell_address(row: usize, col: usize) -> String {
    format!("{}{}", column_to_letter(col), row)
}
