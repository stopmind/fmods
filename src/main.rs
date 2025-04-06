use std::fs::create_dir;
use dirs::config_dir;
use crate::cli::cli;

mod downloader;
mod mod_info;
mod factorio_api;
mod instance;
mod utils;
mod cli;
mod config;

fn main() {
    _ = create_dir(config_dir().unwrap().join("fmods"));

    cli();
}
