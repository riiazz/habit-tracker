use core::panic;

use chrono::{DateTime, Datelike, Utc};
use cliclack::input;
use google_sheets4::{
    Sheets,
    api::{BatchUpdateSpreadsheetRequest, BatchUpdateValuesRequest, ValueRange},
    client,
};
use serde_json::Value;
use yup_oauth2::{
    hyper::{self, client::HttpConnector},
    hyper_rustls::HttpsConnector,
};

use crate::{
    AppConfig,
    data_updater::{get_cell_address, set_data},
    helpers::{
        add_sheet_request, auto_resize_dimension_request, clear_format_request,
        insert_rows_request, repeat_cell_request, set_data_validation_request,
    },
    sheet_parser::{get_active_habits, get_sheet_id},
};

pub async fn generate_template_grid(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    wib: &DateTime<Utc>,
) -> (Vec<Vec<Value>>, i32) {
    let sheet_id = get_sheet_id(hub, app_config, &app_config.sheet_name).await;

    let config_sheet = hub
        .spreadsheets()
        .values_get(&app_config.spreadsheet_id, "Config!A1:C100")
        .doit()
        .await;

    let config_sheet = config_sheet.unwrap();

    let habits = get_active_habits(&config_sheet.1.values.unwrap(), 0);
    let n_row: i32 = (habits.iter().count() + 2) as i32;

    {
        let insert_rows = insert_rows_request(sheet_id, 0, n_row);

        let clear_format = clear_format_request(sheet_id, 0, n_row);

        let month_column_color = repeat_cell_request(
            sheet_id,
            0,
            1,
            0,
            1,
            (1.0, 1.0, 1.0),
            (0.0, 0.0, 0.0),
            10,
            String::from("Arial"),
            String::from("LEFT"),
        );

        let date_column_color = repeat_cell_request(
            sheet_id,
            0,
            1,
            1,
            (wib.num_days_in_month() + 1) as i32,
            (1.0, 1.0, 1.0),
            (0.5, 1.5, 0.5),
            9,
            String::from("Arial"),
            String::from("CENTER"),
        );

        let bool_format = repeat_cell_request(
            sheet_id,
            1,
            n_row - 1,
            1,
            (wib.num_days_in_month() + 1) as i32,
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            9,
            String::from("Arial"),
            String::from("CENTER"),
        );

        let set_cell_data_type = set_data_validation_request(
            sheet_id,
            1,
            n_row - 1,
            1,
            (wib.num_days_in_month() + 1) as i32,
        );

        let update_batch = BatchUpdateSpreadsheetRequest {
            requests: Some(vec![
                insert_rows,
                clear_format,
                set_cell_data_type,
                month_column_color,
                date_column_color,
                bool_format,
            ]),
            include_spreadsheet_in_response: None,
            response_include_grid_data: None,
            response_ranges: None,
        };

        let _ = hub
            .spreadsheets()
            .batch_update(update_batch, &app_config.spreadsheet_id)
            .doit()
            .await;
    }

    let mut sorted_habit: Vec<_> = habits.keys().cloned().collect();
    sorted_habit.sort();

    let mut updated_cell: Vec<ValueRange> = Vec::new();

    let mut row_index: usize = 1;

    let current_month = wib.format("%B").to_string();
    {
        let update_value = String::from(format!("{current_month}"));
        let cell_address = get_cell_address(row_index, 1);
        set_data(
            &mut updated_cell,
            update_value,
            cell_address,
            &app_config.sheet_name,
        );

        row_index += 1;
    }

    for habit in &sorted_habit {
        let update_value = String::from(format!("{habit}"));
        let cell_address = get_cell_address(row_index, 1);
        set_data(
            &mut updated_cell,
            update_value,
            cell_address,
            &app_config.sheet_name,
        );

        row_index += 1;
    }

    for i in 1..wib.num_days_in_month() + 1 {
        let update_value = String::from(format!("{}", i));
        let cell_address = get_cell_address(1, (i + 1) as usize);
        set_data(
            &mut updated_cell,
            update_value,
            cell_address,
            &app_config.sheet_name,
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

    let sheet = hub
        .spreadsheets()
        .values_get(&app_config.spreadsheet_id, &app_config.sheet_name)
        .doit()
        .await;

    let value_range = match sheet {
        Ok((_, value_range)) => {
            println!(
                "✅ '{}' grid created successfully! You’re all set to continue. 🎉",
                current_month
            );
            value_range
                .values
                .expect("Sheet value not found, make sure you have internet connection")
        }
        Err(_) => {
            panic!(
                "Generating new {} grid failed, make sure you have internet connection",
                current_month
            )
        }
    };

    (value_range, sheet_id)
}

pub async fn generate_sheet(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    sheet_name: &str,
    sheet_index: Option<i32>,
) {
    let create_new_sheet = add_sheet_request(sheet_name, sheet_index, 500, 32);

    let update_batch = BatchUpdateSpreadsheetRequest {
        requests: Some(vec![create_new_sheet]),
        include_spreadsheet_in_response: None,
        response_include_grid_data: None,
        response_ranges: None,
    };

    let _ = hub
        .spreadsheets()
        .batch_update(update_batch, &app_config.spreadsheet_id)
        .doit()
        .await;
}

pub async fn auto_resize_dimension(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    sheet_id: i32,
) {
    let resize = auto_resize_dimension_request(sheet_id, "COLUMNS".to_string(), 0, 32);

    let update_batch = BatchUpdateSpreadsheetRequest {
        requests: Some(vec![resize]),
        include_spreadsheet_in_response: None,
        response_include_grid_data: None,
        response_ranges: None,
    };

    let _ = hub
        .spreadsheets()
        .batch_update(update_batch, &app_config.spreadsheet_id)
        .doit()
        .await;
}

pub async fn generate_config_sheet(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
) -> client::Result<(hyper::Response<hyper::body::Body>, ValueRange)> {
    let sheet_name = "Config";
    generate_sheet(hub, app_config, sheet_name, None).await;

    let user_inputs: String = input("Enter you habits (comma separated):")
        .placeholder("e.g. reading, exercise, journaling")
        .validate(|s: &String| {
            if s.trim().is_empty() {
                Err("Please enter at least one habit")
            } else {
                Ok(())
            }
        })
        .interact()
        .unwrap();

    let habits: Vec<String> = {
        let mut inputs: Vec<String> = user_inputs
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        inputs.sort();
        inputs
    };

    let mut updated_cell: Vec<ValueRange> = Vec::new();

    let mut row_index = 1;
    let column_count = {
        let habit_table_column = vec!["Habit", "IsComplete", "IsActive"];
        for (i, column) in habit_table_column.iter().enumerate() {
            let update_value = String::from(format!("{column}"));
            let cell_address = get_cell_address(row_index, i + 1);
            set_data(
                &mut updated_cell,
                update_value,
                cell_address,
                &sheet_name.to_string(),
            );
        }
        row_index += 1;
        habit_table_column.iter().count()
    };

    for habit in &habits {
        let update_value = String::from(format!("{habit}"));
        let cell_address = get_cell_address(row_index, 1);
        set_data(
            &mut updated_cell,
            update_value,
            cell_address,
            &sheet_name.to_string(),
        );

        // set IsActive = true
        let update_value = String::from("TRUE");
        let cell_address = get_cell_address(row_index, 3);
        set_data(
            &mut updated_cell,
            update_value,
            cell_address,
            &sheet_name.to_string(),
        );

        row_index += 1;
    }

    {
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

    let sheet_id = get_sheet_id(hub, app_config, &"Config".to_string()).await;
    let bool_format = set_data_validation_request(
        sheet_id,
        1,
        (habits.iter().count() + 1) as i32,
        1,
        column_count as i32,
    );

    let resize =
        auto_resize_dimension_request(sheet_id, "COLUMNS".to_string(), 0, column_count as i32);

    let update_batch = BatchUpdateSpreadsheetRequest {
        requests: Some(vec![bool_format, resize]),
        include_spreadsheet_in_response: None,
        response_include_grid_data: None,
        response_ranges: None,
    };

    let _ = hub
        .spreadsheets()
        .batch_update(update_batch, &app_config.spreadsheet_id)
        .doit()
        .await;

    let sheet = hub
        .spreadsheets()
        .values_get(&app_config.spreadsheet_id, "Config!A1:C100")
        .doit()
        .await;

    sheet
}
