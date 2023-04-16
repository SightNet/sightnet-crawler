use std::collections::vec_deque::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::{sleep, ThreadId};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use lru_cache::LruCache;
use scraper::Html;
use serde::Deserialize;
use texting_robots::Robot;
use ureq;
use ureq::Error;
use url::Url;

use crate::helper::normalize_url;
use crate::parser::Parser;
use sightnet_core;
use sightnet_core::field::FieldValue;
use sightnet_core::file;

#[derive(Debug, Deserialize, Clone)]
pub struct Cfg {
    pub db_path: String,
    pub user_agent: String,
    pub lru_cache_capacity: usize,
    pub http_reqs_timeout_for_thread: u16,
}

#[derive(Clone)]
pub struct Crawler {
    work: bool,
    sites_queue: VecDeque<String>,
    cache: LruCache<String, bool>,
    threads: Vec<(ThreadId, bool)>,
    cfg: Cfg,
}

impl Crawler {
    pub fn new(cfg: Cfg) -> Self {
        Self {
            work: false,
            sites_queue: VecDeque::new(),
            cache: LruCache::new(cfg.lru_cache_capacity.clone()),
            threads: Vec::new(),
            cfg,
        }
    }

    fn pop_sites(this: Arc<Mutex<Self>>) -> String {
        this.lock().unwrap().sites_queue.pop_front().unwrap()
    }

    fn push_sites(this: Arc<Mutex<Self>>, value: String) {
        this.lock().unwrap().sites_queue.push_back(value);
    }

    fn is_cache_contains(this: Arc<Mutex<Self>>, key: String) -> bool {
        this.lock().unwrap().cache.get_mut(&key).is_some()
    }

    fn put_cache(this: Arc<Mutex<Self>>, key: String, value: bool) {
        this.lock().unwrap().cache.insert(key, value);
    }

    fn has_active_thread(this: Arc<Mutex<Self>>) -> bool {
        let mut has_active_thread = false;

        this.lock().unwrap().threads.iter().for_each(|x| {
            if x.1 {
                has_active_thread = true;
            }
        });

        has_active_thread
    }

    fn set_current_thread_active(this: Arc<Mutex<Self>>, active: bool) {
        for x in this.lock().unwrap().threads.iter_mut() {
            if x.0 == thread::current().id() {
                *x = (x.0, active);
            }
        }
    }

    pub fn process_site(this: Arc<Mutex<Self>>, url: String) {
        let mut db = file::File::load(this.lock().unwrap().cfg.db_path.as_str()).expect("Error while loading!");
        println!("len: {}", db.len());

        let mut url = Url::parse(&url).unwrap();

        let is_cached = Crawler::is_cache_contains(this.clone(), url.as_str().to_string());

        if is_cached {
            info!("Skip {}", url.as_str());
            return;
        }

        let robots_txt_url = url.clone().join("/robots.txt").unwrap();

        let req = ureq::get(robots_txt_url.as_str())
            .set("User-Agent", this.lock().unwrap().cfg.user_agent.as_str())
            .call();
        let res = match req {
            Ok(response) => {
                if response.get_url() != robots_txt_url.as_str() {
                    error!("{} - redirecting from robots.txt is not allowed - {}", url.as_str(), response.get_url());
                    return;
                }

                response.into_string().unwrap()
            }
            Err(Error::Status(code, _response)) => {
                //Crawler::put_cache(this.clone(), url.as_str().to_string(), false);
                error!("{} - {}", robots_txt_url, code);
                return;
            }
            Err(_) => {
                //Crawler::put_cache(this.clone(), url.as_str().to_string(), false);
                error!("{}", robots_txt_url);
                return;
            }
        };

        if res.len() == 0 {
            return;
        }

        let robots_txt = Robot::new(this.lock().unwrap().cfg.user_agent.as_str(), res.as_ref());

        if robots_txt.is_err() {
            error!("{} - while parsing robots txt from website", url.as_str());
            return;
        }

        if !robots_txt.unwrap().allowed(url.as_str()) {
            error!("{} - robots txt disallowed", url.as_str());
            return;
        }

        let req = ureq::get(url.as_str())
            .set("User-Agent", this.lock().unwrap().cfg.user_agent.as_str())
            .call();
        let res = match req {
            Ok(response) => {
                if response.get_url() != url.as_str() {
                    warn!("{} redirected - {}", url.as_str(), response.get_url());
                    url = Url::parse(response.get_url()).unwrap();
                }

                response.into_string().unwrap()
            }
            Err(Error::Status(code, _response)) => {
                Crawler::put_cache(this.clone(), url.as_str().to_string(), false);
                warn!("{} - {}", url, code);
                return;
            }
            Err(_) => {
                Crawler::put_cache(this.clone(), url.as_str().to_string(), false);
                error!("{}", url);
                return;
            }
        };

        if res.len() == 0 {
            return;
        }

        let doc = Html::parse_document(&res);
        let parsed_urls = Parser::parse_urls(&doc).unwrap_or_default();
        let parsed_title = Parser::parse_title(&doc).unwrap_or_default();
        let parsed_desc = Parser::parse_desc(&doc).unwrap_or_default();

        for parsed_url in &parsed_urls {
            let mut parsed_url_obj = url.clone().join(parsed_url).unwrap();
            normalize_url(&mut parsed_url_obj);
            Crawler::push_sites(this.clone(), parsed_url_obj.to_string());
        }

        let mut doc = sightnet_core::document::Document::new();
        let date = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        doc.push("url", FieldValue::from(url.to_string()));
        doc.push("title", FieldValue::from(parsed_title));
        doc.push("desc", FieldValue::from(parsed_desc));
        doc.push("date", FieldValue::from(date as i64));

        db.push(doc, None);

        file::File::save(&db, this.lock().unwrap().cfg.db_path.as_str()).expect("Error while saving!");

        info!("{} - {} links", url.as_str(), parsed_urls.len());

        Crawler::put_cache(this.clone(), url.as_str().to_string(), true);
    }

    pub fn start_threads(this: Arc<Mutex<Self>>, sites: Vec<String>, threads_count: u16) {
        let mut threads = vec![];
        this.lock().unwrap().work = true;

        for site in sites {
            this.lock().unwrap().sites_queue.push_back(site);
        }

        for _ in 0..threads_count {
            let thread = thread::spawn({
                let this_cloned = this.clone();
                move || {
                    Crawler::start_single(this_cloned);
                }
            });

            this.lock().unwrap().threads.push((thread.thread().id(), false));
            threads.push(thread);
        }

        for thread in threads {
            let _ = thread.join();
        }
    }

    pub fn start_single(this: Arc<Mutex<Self>>) {
        let http_reqs_timeout_for_thread = this.lock().unwrap().cfg.http_reqs_timeout_for_thread;

        loop {
            if !this.lock().unwrap().work {
                break;
            }

            if this.lock().unwrap().sites_queue.is_empty() {
                if !Crawler::has_active_thread(this.clone()) {
                    info!("Thread has stopped! Because there are no threads which active(processing a site), and sites list is clear.");
                    break;
                }

                // sleep(std::time::Duration::from_millis(500));
                continue;
            }

            let site = Crawler::pop_sites(this.clone());

            Crawler::set_current_thread_active(this.clone(), true);
            Crawler::process_site(this.clone(), site);
            Crawler::set_current_thread_active(this.clone(), false);

            sleep(std::time::Duration::from_secs(http_reqs_timeout_for_thread.into()));
        }
    }
}
