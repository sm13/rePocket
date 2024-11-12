mod credentials;

use crate::pocketquery::PocketQuery;
use crate::pocketitem::PocketItem;

use reqwest::{StatusCode};
use credentials::Credentials;
use serde_json;
use std::fs::File;
use std::io::Write;

const GET_MURL: &'static str = "https://getpocket.com/v3/get";
const MOD_MURL: &'static str = "https://getpocket.com/v3/send";

#[cfg(not(target_abi = "eabihf"))]
const CREDS_FILE: &'static str = env!("CREDS_FILE_HOST");
#[cfg(target_abi = "eabihf")]
const CREDS_FILE: &'static str = env!("CREDS_FILE_RM");


pub struct Pocket {
    client: reqwest::Client,
    creds: Credentials,
    items_list: Vec<PocketItem>,
    since: u64,
}


impl Pocket {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            creds: Credentials::new(CREDS_FILE),
            items_list: Vec::new(),
            since: 0,
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
            println!("🪼 Reached init() with status {}", val.status());
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


    // Archive one or more items.
    pub async fn archive(&self, items: Vec<u64>) -> Result<reqwest::Response, reqwest::Error> {
        // In the Pocket API, actions is a JSON array of "actions", not confusing at all. Anyways,
        // what that means is that each "action" must have at least 2 fields "action": "archive"
        // and the "item_id": _integer_.

        // For this to work, we need a Value -> Object(Map<String, Value>)
        // The Hash map should be:
        // "actions": [Array]
        // Each array item is:
        // {
        //      "action": "archive"
        //      "item_id": integer
        // }
        // Thus, create a vector of Strings!
        let mut actions: Vec<String> = Vec::<String>::new();

        for id in items {
            actions.push(format!(r#"{{"action": "archive", "item_id": {}}}"#, id));
        }

        // Join all entries of the vector into a single String
        let actions = actions.join(",");
        let actions = actions.trim_end_matches(",");
        let actions = "[".to_string() + actions + "]";
        let actions: serde_json::Value = serde_json::from_str(&actions).unwrap();
        let mut actions: serde_json::Value = serde_json::json!({"actions": actions});
        let c: serde_json::Value = serde_json::json!(self.creds);
        Self::merge_values_into_hashmap(&mut actions, &c);

        let msg = self.client.post(MOD_MURL)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&actions);

        msg.send().await
    }


    // Add one or more tags to an item.
    pub async fn add_tag(&self, item: u64, tags: Vec<String>) -> Result<reqwest::Response, reqwest::Error> {
        let tags: serde_json::Value = serde_json::json!({"action": "tags_add", "item_id": item, "tags": tags.join(",")});
        let mut actions: serde_json::Value = serde_json::json!({"actions": [tags]});
        let c: serde_json::Value = serde_json::json!(self.creds);

        Self::merge_values_into_hashmap(&mut actions, &c);

        let msg = self.client.post(MOD_MURL)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&actions);

        msg.send().await
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


    pub fn since(&self) -> u64 {
        self.since
    }


    fn init_from_json(&mut self, json: serde_json::Value) {
        if env!("VERBOSITY") > "0" {
            println!("🪼 Reached init_from_json()");
            println!("🪼 {:#?}", json["list"]);

            match File::create("response.json") {
                Ok(mut fh) => {
                    writeln!(&mut fh, "{:#?}", json).unwrap();
                },
                Err(err) => println!("🚨 Error!  {:?}", err),
            };
        }


        // This filed is undocumented, however, it is present in the json. This actually makes it
        // all easier!
        self.since = json["since"].as_u64().expect("Expected a timestamp");

        let map = serde_json::Map::from(json["list"].as_object().unwrap().clone());
        for (_, v) in map.iter() {
            if v.is_object() {
                self.items_list.push(serde_json::from_value(v.clone())
                    .expect("🚨 Could not convert this Value to a PocketItem"));
            }
        }
    }


    // Take 2 serde_json::Value and modify the first argument to return a merged HashMap
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
