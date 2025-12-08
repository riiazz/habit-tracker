mod data_updater;
mod helpers;
mod init;
mod interaction;
mod sheet_parser;
mod template_builder;

use crate::{
    data_updater::{bulk_update, get_cell_address, set_data, update_today_progress},
    init::{AppConfig, ensure_sheet_ready, load_app_config, setup_authenticator, valid_months},
    interaction::{get_user_input_exit_session, get_user_input_habit, get_user_inputs},
    sheet_parser::{
        config_table, get_dates, get_habits, get_today_progresses, print_activities,
        print_current_month_total_progress,
    },
};
use chrono::{Duration, Utc};
use cliclack::select;
use core::panic;
use google_sheets4::{
    Sheets,
    api::{BatchUpdateValuesRequest, ValueRange},
};
use std::{collections::HashMap, usize};
use yup_oauth2::{
    hyper::{self},
    hyper_rustls,
};

#[tokio::main]
async fn main() {
    let utc_now = Utc::now();
    let wib = utc_now + Duration::hours(7);
    let date_format = "%Y-%m-%d %H:%M";

    println!("{}", wib.format(date_format));
    println!();

    let app_config: AppConfig = load_app_config(wib).await;

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

    'main_loop: loop {
        let mut values = ensure_sheet_ready(&hub, &app_config, &wib).await;

        let mut months: HashMap<String, usize> = valid_months(&values);
        println!();

        get_today_progresses(&mut values, &mut months, &wib, &hub, &app_config).await;

        println!();

        let mut action_selector = select("How would you like to start?");
        action_selector = action_selector.item(1, "✅ Record today's accomplishments", "");
        action_selector = action_selector.item(2, "🔍 Browse & improve previous entries", "");
        action_selector =
            action_selector.item(3, "dev sandbox, show total progress this month", "");
        action_selector = action_selector.item(4, "🌙 Rest for today (exit)", "");
        action_selector = action_selector.item(5, "dev sandbox, update habit config", "");

        let selected_action = action_selector.interact().unwrap();

        match selected_action {
            1 => {
                update_today_progress(&hub, &app_config, &wib, &mut values, &months).await;
            }
            2 => {
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
            }
            3 => {
                print_current_month_total_progress(&values);
            }
            4 => {
                break 'main_loop;
            }
            5 => {
                let config_sheet_name = "Config";
                let range = format!("{config_sheet_name}!A1:C50");
                let config_sheet = match hub
                    .spreadsheets()
                    .values_get(&app_config.spreadsheet_id, &range)
                    .doit()
                    .await
                {
                    Ok(sheet) => sheet,
                    Err(_) => {
                        panic!("Config sheet not found! Make sure you have internet connection.");
                    }
                };

                let current_month_habits = get_habits(&values, 1);
                let current_month_dates = get_dates(&values, 1);

                let config_values = config_sheet.1.values.expect("Sheet has no values!");

                let config_habits = get_habits(&config_values, config_table::START_ROW_INDEX);
                let mut selected_habits = get_user_input_habit(&config_habits);
                selected_habits.iter_mut().for_each(|(_, v)| *v = true);

                let mut update_value_selector = select("Set habit");
                update_value_selector = update_value_selector.item(true, "Active ✅", "");
                update_value_selector = update_value_selector.item(false, "Inactive 🚫", "");

                let update_value = update_value_selector.interact().unwrap();

                if !update_value {
                    for (habit_name, is_update) in selected_habits.iter_mut() {
                        let cur_month_index = match current_month_habits.get(habit_name) {
                            Some(v) => v,
                            None => continue,
                        };

                        for (_, date_index) in &current_month_dates {
                            if values[*cur_month_index][*date_index] == "TRUE" {
                                *is_update = false;
                                println!(
                                    "{} has activity history. Deactivation is not allowed.",
                                    habit_name
                                );
                                break;
                            }
                        }
                    }
                }

                let update_value = if update_value { "TRUE" } else { "FALSE" };

                let mut updated_cell: Vec<ValueRange> = Vec::new();
                for (habit, is_update) in &selected_habits {
                    if !is_update {
                        continue;
                    }

                    let habit = config_habits.get(habit).unwrap();
                    let cell_address =
                        get_cell_address(*habit + 1, config_table::Column::IsActive.as_usize());

                    set_data(
                        &mut updated_cell,
                        update_value.to_string(),
                        cell_address,
                        &config_sheet_name.to_string(),
                    );
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
                    }
                    Err(err) => {
                        eprint!("Update failed: {:?}", err);
                    }
                }
            }
            _ => unreachable!("Invalid selection"),
        }

        let is_exit = get_user_input_exit_session();

        if is_exit {
            break 'main_loop;
        }
    }

    print!("\nSee you tomorrow!\n");
}
