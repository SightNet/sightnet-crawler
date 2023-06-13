#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::{env, thread};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use config::Config;

use crawler::Cfg;
use crawler::Crawler;

mod parser;
mod crawler;
mod helper;

fn main() {
    env::set_var("RUST_LOG", "crawler");
    env::set_var("RUST_BACKTRACE", "full");

    color_eyre::install().unwrap();
    pretty_env_logger::init();

    let args = env::args().collect::<Vec<String>>();

    let sites_file_path = &args[1];
    let threads_count = &args[2].parse::<u16>().expect("Threads Count should be number value!");

    let sites = fs::read_to_string(sites_file_path)
        .expect(std::format!("file {} doesn't exist!", sites_file_path).as_str())
        .split("\n")
        .map(|x| x.trim())
        .map(String::from)
        .filter(|x| !x.is_empty())
        .collect::<Vec<String>>();

    let cfg = Config::builder()
        .add_source(config::File::with_name("Config.toml"))
        .build()
        .unwrap()
        .try_deserialize::<Cfg>()
        .unwrap();

    let crawler = Crawler::new(cfg.clone());
    let arc = Arc::new(Mutex::new(crawler));

    let _ = thread::spawn(move || {
        loop {
            ureq::get(&format!("{}/col/websites/commit", cfg.db_url)).call().unwrap();
            ureq::get(&format!("{}/col/robots/commit", cfg.db_url)).call().unwrap();
            sleep(Duration::from_secs(20));
        }
    });

    info!("Starting...");
    Crawler::start_threads(arc.clone(), sites, *threads_count);
}
