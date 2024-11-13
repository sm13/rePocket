use serde::{Deserialize, Deserializer};
use serde_json;

// TODO: Consider this for deletion. Right now it serves more as documentation than anything else,
// since the only field in use, namely 'since', I actually read it directly in the pocket mod.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PocketResponse {
    max_actions: usize,
    cachetype: String,
    status: usize,
    complete: usize,
    since: usize,
    list: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PocketItem {
    // Note that it comes in as a string.
    item_id: U64Item,
    // Note that it comes in as a string.
    resolved_id: U64Item,
    resolved_id_str: Option<String>,
    sort_id: Option<usize>,
    // Note that URL is not serde-able.
    given_url: Option<String>,
    resolved_url: Option<String>,
    given_title: Option<String>,
    resolved_title: Option<String>,
    // Ideally, enum like QBool, or a boolean
    favorite: U8Item,
    // Ideally, enum { default, archived, to_delete }
    // Note that the returned json has this as a String!
    status: Option<String>,
    excerpt: Option<String>,
    // Ideally, enum like QBool, or a boolean
    // Note that it comes in as a string.
    is_article: U8Item,
    // Note that it comes in as a string.
    is_index: U8Item,
    // Ideally, enum like QBool, or a boolean
    // Note that it comes in as a string.
    has_image: U8Item,
    // Ideally, enum like QBool, or a boolean
    // Note that it comes in as a string.
    has_video: U8Item,
    // Note that it comes in as a string.
    word_count: U64Item,
    lang: Option<String>,
    tags: Option<serde_json::Value>,
    authors: Option<serde_json::Value>,
    images: Option<serde_json::Value>,
    videos: Option<serde_json::Value>,
    // This is an assumption, I still haven't seen the actual format other than "0"
    //time_favorited: Option<DateTime<Local>>,
    // This fields were not in the documentation!
    //time_added: Option<DateTime<Local>>,
    //time_updated: Option<DateTime<Local>>,
    // This is an assumption, I still haven't seen the actual format other than "0"
    //time_read: Option<DateTime<Local>>,
    // Ideally this would be something to convert into minutes easily, if it isn't already. I
    // haven't seen it. Comes as an integer! May be minutes.
    time_to_read: Option<usize>,
    // Ideally this would be something to convert into minutes easily, if it isn't already. I
    // haven't seen it. Comes as an integer! May be seconds.
    listen_duration_estimate: Option<usize>,

}


impl PocketItem {
    pub fn get_resolved_url(&self) -> Option<String> {
        self.resolved_url.clone()
    }

    pub fn get_resolved_id(&self) -> Option<u64> {
        match self.resolved_id.0 {
            Some(val) => Some(val as u64),
            None => None,
        }
    }

    pub fn get_image_refs(&self) -> Vec<Image> {
        let mut img_list = Vec::<Image>::default();

        let has_image = self.has_image.0.unwrap_or(0);

        if has_image == 1 {
            let map = serde_json::Map::from(self.images.as_ref().expect("Expected an 'images' Object")
                .as_object().unwrap().clone());

            for (_, v) in map.iter() {
                if v.is_object() {
                    img_list.push(serde_json::from_value(v.clone())
                        .expect("🚨 Could not convert this Value to a PocketItem::Image"));
                }
            }
        }

        img_list
    }
}



#[derive(Debug)]
struct U8Item(Option<u8>);

impl<'de> Deserialize<'de> for U8Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if s.is_empty() {
            Ok(U8Item(None))
        } else {
            let v = s.parse::<u8>().map_err(serde::de::Error::custom)?;
            Ok(U8Item(Some(v)))
        }
    }
}

#[derive(Debug, Clone)]
struct U64Item(Option<u64>);

impl<'de> Deserialize<'de> for U64Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if s.is_empty() {
            Ok(U64Item(None))
        } else {
            let v = s.parse::<u64>().map_err(serde::de::Error::custom)?;
            Ok(U64Item(Some(v)))
        }
    }
}


#[derive(Default, Debug, Deserialize)]
pub struct Image {
    pub image_id: String,
    pub src: String,
    item_id: String,
    width: String,
    height: String,
    caption: String,
    credit: String,
}
