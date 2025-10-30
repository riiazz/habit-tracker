use std::collections::HashMap;

use cliclack::{multiselect, select};
use serde_json::Value;

use crate::sheet_parser::{get_dates, get_habits};

pub fn get_user_input_exit_session() -> bool {
    let mut is_exit_selector = select("Wrap up your session? 📘");
    is_exit_selector = is_exit_selector.item(true, "Yes ✅", "");
    is_exit_selector = is_exit_selector.item(false, "No 🚫", "");
    let is_exit = is_exit_selector.interact().unwrap();
    is_exit
}

pub fn get_user_input_update_value() -> bool {
    let mut update_value_selector = select("Mark this habit as complete or not:");
    update_value_selector = update_value_selector.item(true, "Done ✅", "");
    update_value_selector = update_value_selector.item(false, "Skipped 🚫", "");

    let update_value = update_value_selector.interact().unwrap();
    update_value
}

pub fn get_user_inputs(
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

pub fn get_user_input_date(dates: &HashMap<usize, usize>) -> HashMap<usize, bool> {
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

pub fn get_user_input_habit(habits: &HashMap<String, usize>) -> HashMap<String, bool> {
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
