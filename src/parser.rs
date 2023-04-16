use scraper::{Html, Selector};

pub struct Parser {}

impl Parser {
    pub fn parse_title(doc: &Html) -> Option<String> {
        let selector = Selector::parse("title");

        match selector {
            Ok(selector) => {
                let el = doc.select(&selector).next();

                match el {
                    None => None,
                    Some(_) => Some(el.unwrap().inner_html())
                }
            }
            Err(_) => None
        }
    }
    pub fn parse_desc(doc: &Html) -> Option<String> {
        let selector = Selector::parse(r#"meta[property*="desc"]"#);

        match selector {
            Ok(selector) => {
                let el = doc.select(&selector).next();

                match el {
                    None => None,
                    Some(_) => Some(el.unwrap().value().attr("content").unwrap_or_default().to_string())
                }
            }
            Err(_) => None
        }
    }
    pub fn parse_urls(doc: &Html) -> Option<Vec<&str>> {
        let selector = Selector::parse("a");

        match selector {
            Ok(selector) => {
                let elements = doc.select(&selector);
                let mut res: Vec<&str> = vec![];

                for el in elements {
                    let href = el.value().attr("href");

                    match href {
                        None => { continue; }
                        Some(_) => {
                            res.push(href.unwrap());
                        }
                    }
                }

                Some(res)
            }
            Err(_) => None
        }
    }
}