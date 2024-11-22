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

use reqwest;
use std::io;
use std::collections::HashMap;
use std::process::Command;
use std::convert::Infallible;
use serde::Deserialize;
use tokio::sync::oneshot;
use warp::Filter;
use warp::http::StatusCode;

use crate::REDIRECT_URI;

// Pocket Authentication API
const REQUEST_URL:    &'static str = "https://getpocket.com/v3/oauth/request";
const AUTH_URL:       &'static str = "https://getpocket.com/v3/oauth/authorize";
const USER_AUTH_BURL: &'static str = "https://getpocket.com/auth/authorize";
const CERT_PATH:      &'static str = concat!(env!("CERT_DIR"), "/",  "rePocket.crt");
const KEY_PATH:       &'static str = concat!(env!("CERT_DIR"), "/",  "rePocket.key");



#[derive(Default, Clone)]
pub struct PocketAuth {
    pub client: reqwest::Client,
    pub consumer_key: String,
    pub request_token: Option<PocketCode>,
    pub credentials: Option<PocketUser>,
    pub authorized: Box<bool>,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct PocketCode {
    pub code: String,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct PocketUser {
    pub access_token: String,
    pub username: String,
}


impl PocketAuth {
    pub fn new(key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            consumer_key: key.to_string(),
            request_token: None,
            credentials: None,
            authorized: Box::new(false),
        }
    }

    pub async fn connect(&mut self) -> Result<Option<PocketUser>, reqwest::Error> {
        // Get the request token
        let res = self.obtain_request_token().await;

        match res {
            Ok(val)   => {
                let json = val.json::<PocketCode>().await;
                let code = json.unwrap().code;
                self.set_request_token(code);
            },
            Err(e)  => println!("🚨 {e}"),
        }

        println!("{CERT_PATH}");
        println!("{KEY_PATH}");

        let (tx, rx) = oneshot::channel::<bool>();
        let routes = self.get_uri_sink();
        let (_, server) = warp::serve(routes)
            .tls()
            .cert_path(CERT_PATH)
            .key_path(KEY_PATH)
            .bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), async {
                rx.await.ok();
            });
        tokio::task::spawn(server); 
        let _ = self.redirect_user_for_auth(tx).await;
        let aures = self.get_user_approval().await;

        match aures {
            Ok(val)   => {
                let status = val.status();
                let json = val.json::<PocketUser>().await;
                match json {
                    Ok(res) => {
                        self.set_access_creds(res.access_token, res.username); 
                        Ok(self.get_credentials())
                    },
                    Err(e) => {
                        println!("🚨 Couldn't parse the json response with status: '{status}' as credentials. {e}");
                        Err(e)
                    }
                }
            }
            Err(e)  => {
                println!("🚨 Failed while obtaining user approval: {e}");
                Err(e)
            }
        }

    }


    fn set_request_token(&mut self, rtok: String) {
        self.request_token = Some(PocketCode {code: rtok});
    }


    fn get_request_token(&self) -> String {
        self.request_token.clone().unwrap().code
    }


    fn set_access_creds(&mut self, atok: String, uname: String) {
        self.credentials = Some(
            PocketUser {
                access_token: atok,
                username: uname,
            }
        );
    }

    fn get_credentials(&self) -> Option<PocketUser> {
        self.credentials.clone()
    }

    fn get_uri_sink(&self) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let pok = self.authorized.clone();
        // Start the server first.
        warp::path!("pocketapp112512:authorizationFinished")
            .and(warp::any().map(move || pok.clone()))
            .and_then(Self::set_authorized)
    }

    async fn set_authorized(mut pok: Box<bool>) -> Result<impl warp::Reply, Infallible> {
        *pok = true;
        println!("ℹ Authorized, please press <Enter> to continue...");

        Ok(StatusCode::CREATED)
    }


    // To begin the Pocket authorization process, your application must obtain a request token from
    // our servers by making a POST request.
    async fn obtain_request_token(&self) -> Result<reqwest::Response, reqwest::Error> {
        let mut body = HashMap::new();
        body.insert("consumer_key", self.consumer_key.clone());
        body.insert("redirect_uri", REDIRECT_URI.to_string());

        let msg = self.client.post(REQUEST_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/json; charset=UTF8")
            .header("X-Accept", "application/json")
            .json(&body);

        let res = msg.send().await;

        res
    }


    fn get_browser_auth_url(&self) -> Option<String> {
        // TODO: Return a Result, which means create an Error type.
        match &self.request_token {
            Some(token) => {
                Some(format!("{base}?request_token={rtok}&redirect_uri={ruri}",
                    base = USER_AUTH_BURL,
                    rtok = token.code,
                    ruri = REDIRECT_URI
                ))
            }

            None => {
                println!("🚨 call obtain_request_token successfully first.");
                None
            }
        }
    }


    async fn redirect_user_for_auth(&self, tx: oneshot::Sender<bool>) {
        let user_auth_url = self.get_browser_auth_url().unwrap();

        println!("ℹ Redirecting to {user_auth_url} for App authorization. Authorize in the browser");

        // Open the browser (works in macos only, for now).
        let program;

        if cfg!(target_os = "macos") {
            program = "open";
        } else {
            program = "xdg-open";
            println!("🚨 Hmmm, this is not a Mac so I may not know how to open a URL");
        }

        Command::new(program)
            .arg(user_auth_url)
            .spawn()
            .expect("🚨 Failed to open a browser");

        let mut lin = String::new();

        io::stdin().read_line(&mut lin)
            .expect("🚨 Can't parse provided input");

        let _ = tx.send(true);
    }


    // The final step to authorize Pocket with your application is to convert the request token
    // into a Pocket access token. The Pocket access token is the user specific token that you will
    // use to make further calls to the Pocket API.  When your application receives the callback to
    // the redirect_uri supplied in /v3/oauth/request (step 4), you should present some UI to
    // indicate that your application is logging in and make a POST request.
    async fn get_user_approval(&self) -> Result<reqwest::Response, reqwest::Error> {
        println!("ℹ Requesting an access token");

        self.get_access_token().await
    }


    async fn get_access_token(&self) -> Result<reqwest::Response, reqwest::Error> {
        let mut body = HashMap::new();
        body.insert("consumer_key", self.consumer_key.clone());
        body.insert("code", self.get_request_token());

        let msg = self.client.post(AUTH_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/json; charset=UTF8")
            .header("X-Accept", "application/json")
            .json(&body);

        msg.send().await
    }
}
