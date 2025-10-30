mod data_updater;
mod helpers;
mod init;
mod interaction;
mod sheet_parser;
mod template_builder;
use std::{collections::HashMap, usize};

use chrono::{Duration, Utc};
use cliclack::select;
use google_sheets4::Sheets;
use yup_oauth2::{
    hyper::{self},
    hyper_rustls,
};

use crate::{
    data_updater::{bulk_update, update_today_progress},
    init::{AppConfig, load_app_config, setup_authenticator},
    interaction::{get_user_input_exit_session, get_user_inputs},
    sheet_parser::{get_today_progresses, print_activities},
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
        action_selector = action_selector.item(4, "🌙 Rest for today (exit)", "");

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
                generate_template_grid(&hub, &app_config, &wib).await;
            }
            4 => {
                break 'main_loop;
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
