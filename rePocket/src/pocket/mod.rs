mod credentials;

use crate::pocketquery::PocketQuery;
use crate::pocketitem::PocketItem;

use reqwest::{StatusCode};
use credentials::Credentials;
use serde_json;
use std::fs::File;
use std::io::Write;

const GET_MURL: &'static str = "https://getpocket.com/v3/get";

#[cfg(not(target_abi = "eabihf"))]
const CREDS_FILE: &'static str = env!("CREDS_FILE_HOST");
#[cfg(target_abi = "eabihf")]
const CREDS_FILE: &'static str = env!("CREDS_FILE_RM");


pub struct Pocket {
    client: reqwest::Client,
    creds: Credentials,
    items_list: Vec<PocketItem>,
}


impl Pocket {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            creds: Credentials::new(CREDS_FILE),
            items_list: Vec::new(),
        }
    }

    pub async fn retrieve(&self, query: &PocketQuery) -> Result<reqwest::Response, reqwest::Error> {
        let     c: serde_json::Value = serde_json::json!(self.creds);
        let mut q: serde_json::Value = serde_json::json!(query);
        
        Self::merge_values_into_hashmap(&mut q, &c);

        if env!("VERBOSITY") > "0" {
            println!("🪼 Query =>\n{:#?}", q);
        }

        let msg = self.client.post(GET_MURL)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&q);

        msg.send().await
    }


    // Can this be substituted for a trait implementation?
    // Also, substitute the () output for something proper, like Result<_, Error>
    pub async fn init(&mut self, val: reqwest::Response) {
        if env!("VERBOSITY") > "0" {
            println!("🪼 Reached init()");
        }
        match val.status() {
            StatusCode::OK => {
                // It's a kind of magic
                self.init_from_json(val.json().await.unwrap());
            },
            StatusCode::BAD_REQUEST => {
                println!("🚨 Invalid request, please make sure you follow the documentation for proper syntax");
            },
            StatusCode::UNAUTHORIZED => {
                println!("🚨 Problem authenticating the user");
            },
            StatusCode::FORBIDDEN => {
                println!("🚨 User was authenticated, but access denied due to lack of permission or rate limiting");
            },
            StatusCode::SERVICE_UNAVAILABLE => {
                println!("🚨 Pocket's sync server is down for scheduled maintenance");
            },
            _ => {
                println!("🚨 Unkown error encountered");
            },
        }
    }
    
    #[allow(dead_code)]
    pub fn get_urls(&self) -> Option<Vec<String>> {
        let mut urls = Vec::<String>::new();

        for item in self.items_list.iter() {
            match item.get_resolved_url() {
                Some(url) => urls.push(url),
                None => (),
            }
        }

        if urls.is_empty() {
            None
        } else {
            Some(urls)
        }
    }

    fn init_from_json(&mut self, json: serde_json::Value) {
        if env!("VERBOSITY") > "0" {
            println!("🪼 Reached init_from_json()");
            println!("🪼 {:#?}", json["list"]);
        }

        match File::create("response.json") {
            Ok(mut fh) => {
                writeln!(&mut fh, "{:#?}", json).unwrap();
            },
            Err(err) => println!("🚨 Error!  {:?}", err),
        };


        let map = serde_json::Map::from(json["list"].as_object().unwrap().clone());
        for (_, v) in map.iter() {
            if v.is_object() {
                self.items_list.push(serde_json::from_value(v.clone())
                    .expect("🚨 Could not convert this Value to a PocketItem"));
            }
        }
    }


    // Take 2 serde_json::Value and return a HashMap for a reqwest.json()
    fn merge_values_into_hashmap(vj: &mut serde_json::Value, wj: &serde_json::Value) {
        let n = wj.as_object().unwrap();
        let m = vj.as_object_mut().unwrap();

        for (k, v) in n.iter() {
            m.insert(k.clone(), v.clone());
        }
    }
}

use std::ops::Deref;

impl Deref for Pocket {
    type Target = Vec<PocketItem>;

    fn deref(&self) -> &Self::Target {
        &self.items_list
    }
}
