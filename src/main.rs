#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod ui;

fn main() {
    let _cfg = config::Config::default();
    println!("wconfig: hello");
}
