use std::{
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

static APP_USER_AGENT: &str = "rePocket/v0.1.0";

#[derive(Clone)]
pub struct ArticleHandler<'a> {
    item: &'a PocketItem,
    url: String,
    uuid: Uuid,
    page_title: String,
    article_title: String,
    author: String,
    header: String,
    description: String,
    content: String,
    canonical: Option<String>,
}


impl<'a> ArticleHandler<'a> {
    pub fn new(item: &'a PocketItem) -> Self {
        let url = item.get_resolved_url().expect("🚨 No URL found");

        Self {
            item: item,
            uuid :Uuid::new_v5(&Uuid::NAMESPACE_OID, url.as_bytes()),
            url: url.to_string(),
            page_title: String::new(),
            article_title: String::new(),
            author: String::new(),
            header: String::new(),
            description: String::new(),
            content: String::new(),
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

    pub async fn get_readable(&mut self) -> Result<String, (String, StatusCode)> {
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
            .map_err(move |e| {
                // TODO: turn these lines into a little function
                let mut handle = Self::new(p);
                handle.page_title = "The article domain won't talk to me!".to_string();
                handle.article_title = "The article domain won't talk to me!".to_string(); 
                handle.header = "I wonder if I need to try a different approach?".to_string();
                handle.content = format!("Can't fetch URL: {e}");
                handle.canonical = None;

                (handle.html(), StatusCode::BAD_REQUEST)
            })?
            .text()
            .await
            .map_err(move |e| {
                let mut handle = Self::new(p);
                handle.page_title = "I can't get the text for you!".to_string();
                handle.article_title = "I can't get the text for you!".to_string(); 
                handle.header = "Couldn't render article. (It is an article, right?)".to_string();
                handle.content = format!("Can't fetch response body text: {e}");
                handle.canonical = None;

                (handle.html(), StatusCode::BAD_REQUEST)
            })?;

        let url = Url::parse(&self.url).unwrap();
        let (content, meta) = readable_readability::Readability::new().base_url(Some(url.clone())).parse(&body);
        let mut content_bytes = vec![];

        
        content.serialize(&mut content_bytes)
            .map_err(move |e| {
                let mut handle = Self::new(p);
                handle.page_title = "I just can't deal with this!".to_string();
                handle.article_title = "I just can't deal with this!".to_string(); 
                handle.header = "Couldn't extract content form the article. (It is an article, right?)".to_string();
                handle.content = format!("Can't serialize content: {e}");
                handle.canonical = None;

                (handle.html(), StatusCode::BAD_REQUEST)
            })?;


        self.content = std::str::from_utf8(&content_bytes)
            .map_err(move |e| {
                let mut handle = Self::new(p);
                handle.page_title = "I give up...".to_string();
                handle.article_title = "I give up...".to_string(); 
                handle.header = "Invalid UTF-8 in article content".to_string();
                handle.content = format!("Can't serialize content: {e}");
                handle.canonical = None;

                (handle.html(), StatusCode::BAD_REQUEST)
            })?
            .to_string();

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
        .map_err(move |e| {
            let mut handle = Self::new(p);
            handle.page_title = "I just can't deal with this!".to_string();
            handle.article_title = "I just can't deal with this!".to_string(); 
            handle.header = "Couldn't extract content form the article. (It is an article, right?)".to_string();
            handle.content = format!("Can't serialize content: {e}");
            handle.canonical = None;

            (handle.html(), StatusCode::BAD_REQUEST)
        })?;

        if body.content.len() > self.content.len() {
            println!("ℹ Modifying content from readable's readability to readability's extractor");
            self.content = body.content;
        }

        self.content = Self::cleanup_html(&self.content.clone());

        Ok(self.html())
    }


    pub async fn save_file(&mut self, file_type: &str, path: &str) {
        // TODO: This should probably return a -> Result<(), Error>
        let ftype = match file_type {
            "epub" | "pdf" | "html" => file_type,
            // Default to epub, just because
            _ => "epub",
        };

        let res = self.get_readable().await;

        match res {
            Ok(article) => {
                match File::create(format!("{}/{}.{}", path, self.uuid, ftype)) {
                    Ok(mut fh) => {
                        match ftype {
                            "epub" => {
                                let _ = fh.write_all(&self.epub());
                            },
                            "html" => {
                                let _ = fh.write_all(&article.as_bytes());
                            },
                            _ => {
                                println!("ℹ Not saving file! Only \"pdf\" and \"epub\" supported");
                            },
                        }
                    },
                    Err(err) => println!("🚨 Error creating file!  {:?}", err),
                }
            },
            Err(err) => println!("🚨 Error getting readable  {:?}", err),
        }
    }


    fn epub(&self) -> Vec<u8> {
        let mut builder = EpubBuilder::new(ZipLibrary::new().unwrap()).unwrap();

        builder.metadata("title", format!("{}", self.article_title)).unwrap();
        builder.metadata("author", format!("{}", self.author)).unwrap();
        builder.metadata("description", format!("{}", self.description)).unwrap();
        builder.epub_version(epub_builder::EpubVersion::V30);
        builder.add_content(epub_builder::EpubContent::new("article.xhtml", self.html().as_bytes())
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


    fn html(&self) -> String {
        let template = include_str!("../../data/template.html");
        let mut output = template
            .replace("{{page_title}}", &self.page_title)
            .replace("{{article_title}}", &self.article_title)
            .replace("{{header}}", &self.header)
            .replace("{{content}}", &self.content);

        if let Some(canonical) = &self.canonical {
            output = output.replace(
                "{{canonical}}",
                &format!("<link rel=\"canonical\" href=\"{canonical}\" />"),
            );
        } else {
            output = output.replace("{{canonical}}", "");
        }

        output
    }


    // Clean up the HTML to make it more like XHTML. ammonia will do the heavy lifting,
    // however, it is not a conversion tool. I decided against all other solutions because
    // they eitehr didn't really work or dpended on an external tool.
    //
    // Certain things need a different approach. Unfortunately, for this I'm down to string
    // substitution and regexes. I know, I know, ...
    fn cleanup_html(html: &str) -> String {
        // TODO: When implementing pictures in the epubs, either substitute the photos
        // for the alt text, or download the photos and add relative links. Either way,
        // this will probably have to go.
        let output = ammonia::Builder::default()
            .rm_tags(&["div"])
            .rm_tag_attributes("img", &["alt"])
            .clean(html)
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
