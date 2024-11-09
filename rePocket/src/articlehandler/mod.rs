use std::{
    error,
    fmt,
    include_str,
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
            self.page_title = meta.page_title.unwrap_or_else(|| "Page".into());
            self.article_title = meta.article_title.unwrap_or_else(|| "Article".into());
            self.description = meta.description.unwrap_or_else(|| "Description".into());
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

            self.content = Self::cleanup_html(&self.content.clone());

            Ok(self.html())
        }
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
                                let _ = fh.write_all(&self.epub());
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


    fn epub(&self) -> Vec<u8> {
        let mut builder = EpubBuilder::new(ZipLibrary::new().unwrap()).unwrap();

        builder.metadata("title", format!("{}", self.article_title)).unwrap();
        builder.metadata("author", format!("{}", self.author)).unwrap();
        builder.metadata("description", format!("{}", self.description)).unwrap();
        builder.epub_version(epub_builder::EpubVersion::V30);
        builder.add_content(epub_builder::EpubContent::new("article.xhtml", self.html().as_slice())
            .title(format!("{}", self.article_title))
            .reftype(epub_builder::ReferenceType::Text)).unwrap();
        //builder.add_cover_image().unwrap();

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
    fn cleanup_html(html: &Vec<u8>) -> Vec<u8> {
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
        let re = Regex::new(r"<source(.*?)>").unwrap();
        let output = re.replace_all(&output, "<source$1 />")
            // Fixes an issue with remarkable not liking the tag
            .replace("<img />", "")
            // This is to make XHTML happy
            .replace("<hr>", "<hr />");

        output.into()
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
