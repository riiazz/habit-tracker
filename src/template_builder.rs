use chrono::{DateTime, Datelike, Utc};
use google_sheets4::{
    Sheets,
    api::{BatchUpdateSpreadsheetRequest, BatchUpdateValuesRequest, ValueRange},
};
use serde_json::Value;
use yup_oauth2::{hyper::client::HttpConnector, hyper_rustls::HttpsConnector};

use crate::{
    AppConfig,
    data_updater::{cell_address, set_data},
    helpers::{
        add_sheet_request, auto_resize_dimension_request, clear_format_request,
        insert_rows_request, repeat_cell_request, set_data_validation_request,
    },
    sheet_parser::get_active_habits,
};

pub async fn generate_template_grid(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    app_config: &AppConfig,
    wib: &DateTime<Utc>,
) -> (Vec<Vec<Value>>, i32) {
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
                .find(|props| props.title.as_deref() == Some(&app_config.sheet_name))
                .and_then(|props| props.sheet_id)
        })
        .unwrap_or_else(|| panic!("Sheet id for {} not found!", &app_config.sheet_name));

    let config_sheet = hub
        .spreadsheets()
        .values_get(&app_config.spreadsheet_id, "Config!A1:C100")
        .doit()
        .await;

    let config_sheet = config_sheet
        .unwrap_or_else(|_| panic!("Config sheet not found!"))
        .1
        .values
        .unwrap_or_default();

    let habits = get_active_habits(&config_sheet, 0);
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
            ..BatchUpdateSpreadsheetRequest::default()
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
        let cell_address = cell_address(row_index, 1);
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
        let cell_address = cell_address(row_index, 1);
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
        let cell_address = cell_address(1, (i + 1) as usize);
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

pub async fn generate_sheet(hub: &Sheets<HttpsConnector<HttpConnector>>, app_config: &AppConfig) {
    let create_new_sheet = add_sheet_request(app_config.sheet_name.as_str(), Some(0), 500, 32);

    let update_batch = BatchUpdateSpreadsheetRequest {
        requests: Some(vec![create_new_sheet]),
        ..BatchUpdateSpreadsheetRequest::default()
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
        ..BatchUpdateSpreadsheetRequest::default()
    };

    let _ = hub
        .spreadsheets()
        .batch_update(update_batch, &app_config.spreadsheet_id)
        .doit()
        .await;
}
