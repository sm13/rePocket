//
// Copyright (c) 2024 Damián Sánchez Moreno
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.
//

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
