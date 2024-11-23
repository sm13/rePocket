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

use std::{
    error,
    fmt,
    include_str,
    collections::HashMap,
    time::Duration,
    fs::File,
    io::Write,
};
use url::Url;
use ammonia;
use regex::Regex;
use uuid::Uuid;
use reqwest::StatusCode;
use readable_readability;
use readability;
use epub_builder::{
    EpubBuilder,
    ZipLibrary,
};

use crate::pocketitem::PocketItem;
use crate::utils;

static APP_USER_AGENT: &str = "rePocket/v0.2.0";


#[derive(Debug)]
enum Error {
    IO(std::io::Error),
    Reqwest(reqwest::Error),
    HeaderToStr(reqwest::header::ToStrError),
    Readability(readability::error::Error),
    Tokio(tokio::task::JoinError),
}


#[derive(Clone)]
pub struct ArticleHandler<'a> {
    item: &'a PocketItem,
    url: String,
    is_pdf: bool,
    uuid: Uuid,
    page_title: String,
    article_title: String,
    author: String,
    header: String,
    description: String,
    content: Vec<u8>,
    canonical: Option<String>,
    images: HashMap<String, String>,
}


impl<'a> ArticleHandler<'a> {
    pub fn new(item: &'a PocketItem) -> Self {
        let url = item.get_resolved_url().expect("🚨 No URL found");

        Self {
            item: item,
            url: url.to_string(),
            is_pdf: false,
            uuid :Uuid::new_v5(&Uuid::NAMESPACE_OID, url.as_bytes()),
            page_title: String::new(),
            article_title: String::new(),
            author: String::new(),
            header: String::new(),
            description: String::new(),
            content: Vec::<u8>::new(),
            canonical: None,
            images: Self::image_list(item),
        }
    }

    pub fn title(&self) -> String {
        self.article_title.clone()
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn uuid_string(&self) -> String {
        utils::uuid_to_string(self.uuid)
    }

    pub async fn get_readable(&mut self) -> Result<Vec<u8>, (Vec<u8>, StatusCode)> {
        // Looks like to get responses from some servers it is necessary to include the user_agent()
        let client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::new(30, 0))
            .build();

        let p = &self.item;

        let body = client.expect("🚨 Cannot open reqwest client")
            .get(self.url.clone())
            .send()
            .await
            .map_err(move |e| { Self::error_html(p, Error::Reqwest(e)) })?;

        // Check the response for content-type, and treat PDF differently.
        if body.headers()["content-type"] == "application/pdf" {
            let body = body.bytes()
                .await
                .map_err(move |e| { Self::error_html(p, Error::Reqwest(e)) })?;

            self.is_pdf = true;
            self.content = body.to_vec();

            Ok(body.to_vec())

        } else {
            // TODO:
            // 1) Make this a function?
            // 2) Can I move all the map_err to a single location? This is waaay toooo loooong
            let body = body.text()
            .await
            .map_err(move |e| { Self::error_html(p, Error::Reqwest(e)) })?;

            let url = Url::parse(&self.url).unwrap();
            let (content, meta) = readable_readability::Readability::new().base_url(Some(url.clone())).parse(&body);
            let mut content_bytes = vec![];

            content.serialize(&mut content_bytes)
                .map_err(move |e| { Self::error_html(p, Error::IO(e)) })?;

            self.content = content_bytes;

            self.header = format!(
                "A rePocket-able version of <a class=\"shortened\" href=\"{url}\">{url}</a><br />Retrieved on {}",
                Self::now_string()
            );

            // If some fields are missing fill them with some defaults.
            self.author = meta.byline.unwrap_or_else(|| "Unknown".into());
            self.page_title = Self::encode_text(&meta.page_title.unwrap_or_else(|| "Page".into()));
            self.article_title = Self::encode_text(&meta.article_title.unwrap_or_else(|| "Article".into()));
            self.description = Self::encode_text(&meta.description.unwrap_or_else(|| "Description".into()));
            self.canonical = Some(url.to_string());

            // Some websites appear empty or very short using readable::readability.
            // Thus, also obtain them with readability::extractor to choose the best one.
            // What "best" means is open to interpretation, for the time being, longer is better.
            let blocking_url = url.clone();

            let body = tokio::task::spawn_blocking(move || {
                match readability::extractor::scrape(&blocking_url.to_string()) {
                    Ok(body) => body,
                    Err(e)   => readability::extractor::Product {
                        title: "readability::extractor didn't work".to_string(),
                        content: format!("<p>readability::extractor didn't work: {e}</p>"),
                        text: format!("readability::extractor didn't work: {e}"),
                    },
                }
            }).await
            .map_err(move |e| { Self::error_html(p, Error::Tokio(e)) })?;

            if body.content.len() > self.content.len() {
                println!("ℹ Modifying content from readable's readability to readability's extractor");
                self.content = body.content.into();
            }

            self.image_list_all().await;
            self.content = self.cleanup_html(&self.content.clone());

            Ok(self.html())
        }
    }


    async fn get_image(url: &str) -> Result<(Vec<u8>, String), Error> {
        let client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::new(30, 0))
            .build();

        let body = client.expect("🚨 Cannot open reqwest client")
            .get(url)
            .send()
            .await
            .map_err(|e| { Error::Reqwest(e) })?;

        let mime_type = body.headers()["content-type"].to_str()?.to_string();

        let body = body.bytes()
            .await
            .map_err(|e| { Error::Reqwest(e) })?;

        Ok((body.to_vec(), mime_type))
    }



    fn error_html(item: &'a PocketItem, e: Error) -> (Vec<u8>, StatusCode) {
        // TODO: turn these lines into a little function
        let mut handle = Self::new(item);
        handle.page_title = "rePocket Failed!".to_string();
        handle.article_title = "".to_string();
        handle.header = "Could not get the article contents".to_string();
        handle.content = format!("Could not get the article contents. Reason:\n{e}").into();
        handle.canonical = None;

        (handle.html(), StatusCode::BAD_REQUEST)
    }


    pub async fn save_file(&mut self, file_type: &str, path: &str) {
        // TODO: This should probably return a -> Result<(), Error>
        let res = self.get_readable().await;

        let mut ftype = match file_type {
            "epub" | "pdf" | "html" => file_type,
            // Default to epub, just because
            _ => "epub",
        };

        if self.is_pdf {
            ftype = "pdf";
        }

        match res {
            Ok(article) => {
                match File::create(format!("{}/{}.{}", path, self.uuid, ftype)) {
                    Ok(mut fh) => {
                        match ftype {
                            "epub" => {
                                let _ = fh.write_all(&self.epub().await);
                            },
                            "html" => {
                                let _ = fh.write_all(&article);
                            },
                            "pdf" => {
                                let _ = fh.write_all(&self.content);
                            }
                            _ => {
                                println!("ℹ Not saving file! Only \"pdf\", \"html\" and \"epub\" supported");
                            },
                        }
                    },
                    Err(err) => println!("🚨 Error creating file! {:?}", err),
                }
            },
            Err(err) => println!("🚨 Error getting readable {:?}", err),
        }
    }


    async fn epub(&self) -> Vec<u8> {
        let mut builder = EpubBuilder::new(ZipLibrary::new().unwrap()).unwrap();

        builder.metadata("title", format!("{}", self.article_title)).unwrap();
        builder.metadata("author", format!("{}", self.author)).unwrap();
        builder.metadata("description", format!("{}", self.description)).unwrap();
        builder.epub_version(epub_builder::EpubVersion::V30);
        builder.add_content(epub_builder::EpubContent::new("article.xhtml", self.html().as_slice())
            .title(format!("{}", self.article_title))
            .reftype(epub_builder::ReferenceType::Text)).unwrap();

        // Add images.
        let mut set_cover = true;
        for (url, loc) in &self.images {
            let res = Self::get_image(&url).await;

            let (bin, mime_type) = res.expect("Expected bin and mime_type");

            builder.add_resource(&loc, &*bin, mime_type.clone()).unwrap();

            if set_cover {
                set_cover = false;
                // Add cover image
                builder.add_cover_image(&loc, &*bin, mime_type).unwrap();
            }
        }


        let mut epub: Vec<u8> = vec!();

        match builder.generate(&mut epub) {
            Ok(())    => (),
            Err(e)  => println!("🚨 Can't build epub: {e}"),
        }

        epub
    }


    fn html(&self) -> Vec<u8> {
        let template = include_str!("../../data/template.html");
        let mut output = template
            .replace("{{page_title}}", &self.page_title)
            .replace("{{article_title}}", &self.article_title)
            .replace("{{header}}", &self.header)
            .replace("{{content}}", &String::from_utf8(self.content.clone()).unwrap());

        if let Some(canonical) = &self.canonical {
            output = output.replace(
                "{{canonical}}",
                &format!("<link rel=\"canonical\" href=\"{canonical}\" />"),
            );
        } else {
            output = output.replace("{{canonical}}", "");
        }

        output.into()
    }


    // Clean up the HTML to make it more like XHTML. ammonia will do the heavy lifting,
    // however, it is not a conversion tool. I decided against all other solutions because
    // they eitehr didn't really work or dpended on an external tool.
    //
    // Certain things need a different approach. Unfortunately, for this I'm down to string
    // substitution and regexes. I know, I know, ...
    fn cleanup_html(&self, html: &Vec<u8>) -> Vec<u8> {
        // TODO: When implementing pictures in the epubs, either substitute the photos
        // for the alt text, or download the photos and add relative links. Either way,
        // this will probably have to go.
        let dirty = &String::from_utf8(html.to_vec()).unwrap();

        let output = ammonia::Builder::default()
            .rm_tags(&["div"])
            .rm_tag_attributes("img", &["alt"])
            .clean(dirty)
            .to_string();

        let re = Regex::new(r"<img(.*?)>").unwrap();
        let output = re.replace_all(&output, "<img$1 />");
        let re = Regex::new(r"<map>.*?</map>").unwrap();
        let output = re.replace_all(&output, "");
        let re = Regex::new(r"<source(.*?)>").unwrap();
        let mut output = re.replace_all(&output, "<source$1 />")
            // Fixes an issue with remarkable not liking the tag, as in make it XTHML
            .replace("<img />", "")
            // This is to make XHTML happy
            .replace("<hr>", "<hr />")
            // This is also to make XHTML happy
            .replace("<br>", "<br />");

        // Fix images (or attempt to anyways)
        for (k, v) in &self.images {
            output = output.replace(k, v);
        }

        output.into()
    }


    // This function is similar to (I guess) what html_scape::encode_text() does.
    // In summary, encondes &, >, and < in strings to make them HTML-able.
    fn encode_text(non_html: &str) -> String {
        // This is also to make XHTML happy, or remarkable, I don't even know anymore
        let html = non_html
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;");

        html
    }

    // Get image URLs as Pocket identifies them
    fn image_list(item: &'a PocketItem) -> HashMap<String, String> {
        let mut img_list = HashMap::<String, String>::new();

        img_list
    }


    // Get image URLs from the HTML, and save them into our list with **extensions**
    async fn image_list_all(&mut self) -> Result<(), Error> {
        // First find the images in the HTML.
        let re = Regex::new(r#"<img.*?src="(?<url>.*?)".*?>"#).unwrap();
        let cont = String::from_utf8(self.content.clone()).unwrap();
        let imgs = re.captures_iter(&cont);

        let mut client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::new(30, 0))
            .build();

        for img in imgs {
            let url = img["url"].to_string();

            let body = client.as_ref().expect("🚨 Cannot open reqwest client to get Image header")
                .head(&url)
                .send()
                .await
                .map_err(|e| { Error::Reqwest(e) })?;

            let mime_type = body.headers()["content-type"].to_str()?.to_string();
            let (_, ext) = mime_type.rsplit_once("/").expect("Expected a proper mime_type");

            let uuid = utils::uuid_to_string(Uuid::new_v5(&Uuid::NAMESPACE_OID, url.as_bytes()));
            let mut fname = format!("p{}.{}", uuid, ext);

            self.images.insert(url, fname);
        }

        Ok(())
    }


    fn now_string() -> String {
        let now = chrono::Local::now();
        now.format("%Y.%B.%e, %T").to_string()
    }
}


impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl error::Error for Error { }

impl From<std::io::Error> for Error {
  fn from(error: std::io::Error) -> Self {
    Error::IO(error)
  }
}

impl From<reqwest::Error> for Error {
  fn from(error: reqwest::Error) -> Self {
    Error::Reqwest(error)
  }
}

impl From<reqwest::header::ToStrError> for Error {
  fn from(error: reqwest::header::ToStrError) -> Self {
    Error::HeaderToStr(error)
  }
}

impl From<readability::error::Error> for Error {
  fn from(error: readability::error::Error) -> Self {
    Error::Readability(error)
  }
}

impl From<tokio::task::JoinError> for Error {
  fn from(error: tokio::task::JoinError) -> Self {
    Error::Tokio(error)
  }
}
