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
