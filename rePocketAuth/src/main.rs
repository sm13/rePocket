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

mod pocketauth;

use pocketauth::PocketAuth;

use std::io::{
    self,
    Write,
};
use std::fs::File;


const REDIRECT_URI : &'static str = "https://127.0.0.1:3030/pocketapp112512:authorizationFinished";


#[tokio::main]
async fn main() {
    println!("Insert your Consumer Key, e.g. 123456-0123456789abcdef0c0ffee");

    let mut ckey = String::new();
    io::stdin().read_line(&mut ckey).expect("Couldn't parse the provided Consumer Key");

    let mut mypocket = PocketAuth::new(&ckey);

    let res = mypocket.connect().await;

    match res {
        Ok(creds)   => {
            println!("ℹ Writing credentials for user '{:#?}' in {}",
                env!("CREDS_FILE"),
                creds.clone().unwrap().username);

            match File::create(env!("CREDS_FILE")) {
                Ok(mut fh) => {
                    writeln!(&mut fh, "{ckey}").unwrap();
                    writeln!(&mut fh, "{ak}", ak = creds.unwrap().access_token).unwrap();
                },
                Err(err) => println!("🚨 Error!  {:?}", err),
            };
        },
        Err(err)    => println!("🚨 Error!  {:?}", err),
    };
}
