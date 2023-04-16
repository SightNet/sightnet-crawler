#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::env;
use std::fs;
use std::sync::{Arc, Mutex};

use config::Config;
use lazy_static::lazy_static;
use sightnet_core::collection::Collection;
use sightnet_core::field::FieldType;

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

    let db = sightnet_core::file::File::load(cfg.db_path.as_str());

    if db.is_err() {
        //there is no file, we should init collection and save it
        let mut db = Collection::default();

        db.push_field("url", FieldType::String);
        db.push_field("title", FieldType::String);
        db.push_field("desc", FieldType::String);
        db.push_field("date", FieldType::Int);

        sightnet_core::file::File::save(&db, cfg.db_path.as_str()).unwrap();
    }

    let crawler = Crawler::new(cfg);
    let arc = Arc::new(Mutex::new(crawler));

    Crawler::start_threads(arc.clone(), sites, *threads_count);
}
