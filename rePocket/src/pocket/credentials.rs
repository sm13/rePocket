use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::path::Path;
use serde::{Serialize};


#[derive(Serialize)]
pub struct Credentials {
    pub consumer_key: String,
    pub access_token: String,
}


impl Credentials {
    pub fn new(fname: &str) -> Self {
        let fh = match File::open(&Path::new(fname)) {
            Err(why) => panic!("🚨 Couldn't open {fname}: {}", why),
            Ok(file) => file,
        };

        let mut lines = BufReader::new(fh).lines().flatten();

        let (ck, at) = (lines.next(), lines.next());

        Self {
            consumer_key: ck.expect("🚨 Could not parse consumer_key from credentials file."),
            access_token: at.expect("🚨 Could not parse access_token from credentials file."),
        }
    }
}
