use google_sheets4::{
    FieldMask,
    api::{CellData, CellFormat, Color, GridRange, RepeatCellRequest, Request, TextFormat},
};

/// Helper to build a Color
pub fn color(r: f32, g: f32, b: f32) -> Color {
    Color {
        red: Some(r),
        green: Some(g),
        blue: Some(b),
        alpha: Some(1.0),
    }
}

/// Helper to build a RepeatCell Request safely (no recursive defaults)
pub fn repeat_cell_request(
    sheet_id: i32,
    start_row: i32,
    end_row: i32,
    start_col: i32,
    end_col: i32,
    fg: (f32, f32, f32),
    bg: (f32, f32, f32),
    font_size: i32,
    font_family: String,
    horizontal_alignment: String,
) -> Request {
    let text_format = TextFormat {
        foreground_color: Some(color(fg.0, fg.1, fg.2)),
        ..TextFormat {
            bold: None,
            italic: None,
            strikethrough: None,
            underline: None,
            font_size: Some(font_size),
            font_family: Some(font_family),
            link: None,
            foreground_color: None,
            foreground_color_style: None,
        }
    };

    let cell_format = CellFormat {
        text_format: Some(text_format),
        background_color: Some(color(bg.0, bg.1, bg.2)),
        ..CellFormat {
            borders: None,
            horizontal_alignment: Some(horizontal_alignment),
            vertical_alignment: None,
            wrap_strategy: None,
            number_format: None,
            padding: None,
            background_color_style: None,
            hyperlink_display_type: None,
            text_direction: None,
            text_rotation: None,
            text_format: None,
            background_color: None,
        }
    };

    Request {
        repeat_cell: Some(RepeatCellRequest {
            range: Some(GridRange {
                sheet_id: Some(sheet_id),
                start_row_index: Some(start_row),
                end_row_index: Some(end_row),
                start_column_index: Some(start_col),
                end_column_index: Some(end_col),
            }),
            cell: Some(CellData {
                user_entered_format: Some(cell_format),
                ..CellData {
                    effective_value: None,
                    effective_format: None,
                    pivot_table: None,
                    data_source_table: None,
                    data_source_formula: None,
                    text_format_runs: None,
                    hyperlink: None,
                    note: None,
                    user_entered_value: None,
                    user_entered_format: None,
                    data_validation: None,
                    formatted_value: None,
                }
            }),
            fields: Some(FieldMask::new(&vec![
                "userEnteredFormat.backgroundColor".to_string(),
                "userEnteredFormat.textFormat.foregroundColor".to_string(),
                "userEnteredFormat.textFormat.fontSize".to_string(),
                "userEnteredFormat.horizontalAlignment".to_string(),
            ])),
        }),
        ..Request {
            update_embedded_object_border: None,
            update_spreadsheet_properties: None,
            cancel_data_source_refresh: None,
            insert_range: None,
            add_banding: None,
            add_chart: None,
            add_conditional_format_rule: None,
            add_data_source: None,
            add_dimension_group: None,
            add_filter_view: None,
            add_named_range: None,
            add_protected_range: None,
            add_sheet: None,
            append_cells: None,
            append_dimension: None,
            auto_fill: None,
            auto_resize_dimensions: None,
            clear_basic_filter: None,
            copy_paste: None,
            create_developer_metadata: None,
            cut_paste: None,
            delete_banding: None,
            delete_conditional_format_rule: None,
            delete_data_source: None,
            delete_developer_metadata: None,
            delete_dimension: None,
            delete_dimension_group: None,
            delete_duplicates: None,
            delete_embedded_object: None,
            delete_filter_view: None,
            delete_named_range: None,
            delete_protected_range: None,
            delete_range: None,
            delete_sheet: None,
            duplicate_filter_view: None,
            duplicate_sheet: None,
            find_replace: None,
            insert_dimension: None,
            merge_cells: None,
            move_dimension: None,
            paste_data: None,
            randomize_range: None,
            refresh_data_source: None,
            repeat_cell: None,
            set_basic_filter: None,
            set_data_validation: None,
            sort_range: None,
            text_to_columns: None,
            trim_whitespace: None,
            unmerge_cells: None,
            update_banding: None,
            update_borders: None,
            update_cells: None,
            update_chart_spec: None,
            update_conditional_format_rule: None,
            update_data_source: None,
            update_developer_metadata: None,
            update_dimension_group: None,
            update_dimension_properties: None,
            update_embedded_object_position: None,
            update_filter_view: None,
            update_named_range: None,
            update_protected_range: None,
            update_sheet_properties: None,
            update_slicer_spec: None,
            add_slicer: None,
        }
    }
}
