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

use std::collections::{BTreeMap, btree_map};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{BufReader, BufWriter, Write};
use std::fs::read;
use std::fs::File;
use std::str;
use uuid::Uuid;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde_json::json;

use crate::pocketitem::PocketItem;
use crate::articlehandler::ArticleHandler;
use crate::utils;


#[cfg(not(target_abi = "eabihf"))]
pub const XOCHITL_ROOT  : &'static str = concat!(env!("HOME"), "/", ".local/share/remarkable/xochitl");
#[cfg(not(target_abi = "eabihf"))]
pub const CONFIG_FILE   : &'static str = env!("CONFIG_FILE_HOST");

#[cfg(target_abi = "eabihf")]
pub const XOCHITL_ROOT  : &'static str = "/home/root/.local/share/remarkable/xochitl";
#[cfg(target_abi = "eabihf")]
pub const CONFIG_FILE   : &'static str = env!("CONFIG_FILE_RM");


#[derive(Clone, Debug, Serialize, Deserialize)]
// Serialize this into json
pub struct FSHandler {
    // The UUID of the file representing the Pocket folder
    folder: UniqID,
    // The UUID of the file representing the Pocket/Archive folder
    #[serde(default)]
    archive: UniqID,
    // The items currenlty present in the folder
    current_items: BTreeMap<UniqID, u64>,
    // Read and archived items
    archived_items: BTreeMap<UniqID, u64>,
    // Items corresponding to the query from the most recent run
    #[serde(default)]
    new_items: BTreeMap<UniqID, u64>,
    // Read, marked to archived, but not archived.
    #[allow(dead_code)]
    #[serde(skip)]
    read_items: BTreeMap<UniqID, u64>,
    #[serde(default)]
    ts_last_query: u64,
}


//
// Picture this. Configuration files are stored in the user's HOME
//
// /home/root/.repocket/.
//                      ├── .repocket.key
//                      └── repocket.config
//
//
// The files are soved in a Pocket folder at the root of reMarkable's files.
//
// /home/root/**/XOCHITL_ROOT/Pocket/.
//                                   ├── Article1.epub
//                                   ├── Article2.epub
//                                   └── Archive
//                                       └── Article3.epub
//
// Which really looks like this:
// XOCHITL_ROOT/.
//              ├── archive-uuid.content
//              ├── archive-uuid.meta
//              ├── folder-uuid.content
//              ├── folder-uuid.meta
//              ├── Article1-uuid.epub
//              ├── Article1-uuid.content
//              ├── Article1-uuid.metadata
//              |..
//              ├── Articlen-uuid.epub
//              ├── Articlen-uuid.content
//              └── Articlen-uuid.metadata
//
impl FSHandler {
    pub fn new() -> Self {
        Self {
            folder: UniqID::new(),
            archive: UniqID::new(),
            current_items: BTreeMap::new(),
            archived_items: BTreeMap::new(),
            new_items: BTreeMap::new(),
            read_items: BTreeMap::new(),
            ts_last_query: 0,
        }
    }

    pub fn load() -> Self {
        // Read the CONFIG_FILE if it exists,
        let config = read(CONFIG_FILE);

        match config {
            Ok(data) => {
                // Create a Self from the data.
                let mut loaded : Self = serde_json::from_slice(&data).unwrap();

                // TODO: Call consolidate() (perhaps this is the action that we can trigger manually?)
                loaded.consolidate();

                loaded
            },
            Err(_) => {
                // otherwise, call new()
                let new = Self::new();

                // call to create the pocket folder
                let res = new.mkdir_pocket();

                match res {
                    Ok(()) => new,
                    Err(e) => panic!("BAD STUFF {e}"),
                }
            },
        }
    }


    //
    // Write the config file.
    //
    // The config file is really a .json file with this structure:
    // {
    //      "folder": "string",
    //      "archive": "string",
    //      "ts_last_query": integer,
    //      "current_items": {
    //          "string" :integer,
    //          ...
    //          "string" :integer
    //      },
    //      "archived_items": {
    //          "string" :integer,
    //          ...
    //          "string" :integer
    //      }
    // }
    //
    pub fn save_config(&self) {
        match File::create(CONFIG_FILE) {
            Ok(fh) => {
                let buffer = BufWriter::new(fh);
                match serde_json::to_writer(buffer, self) {
                    Ok(()) => (),
                    Err(e) => println!("🚨 Cannot convert string to json! {e}"),
                }
            },
            Err(e) => panic!("🚨 Cannot save configuration file! {e}"),
        }
    }


    pub fn mkdir_pocket(&self) -> Result<(), std::io::Error> {
        // Pocket directory:
        let pocket_res = self.mkdir(&self.parent_uuid_string(), "Pocket", "");

        // Pocket/Archive directory:
        self.mkdir(&self.archive_uuid_string(), "Archive", &self.parent_uuid_string())?;

        pocket_res
    }


    fn mkdir(&self, uuid: &str, name: &str, parent: &str) -> Result<(), std::io::Error> {
        // Create a file with the new UUID
        let fname_content: String = XOCHITL_ROOT.to_string() + "/" + uuid + ".content";
        let mut fh = File::create_new(fname_content)?;
        writeln!(fh, "{{}}")?;

        // Create the metadatafile
        let metadata = Metadata::new("CollectionType", name, parent);
        let json = metadata.json()?;
        let fname_meta = XOCHITL_ROOT.to_string() + "/" + uuid + ".metadata";
        let mut fh = File::create_new(fname_meta)?;
        writeln!(fh, "{}", json)?;

        Ok(())
    }


    pub async fn new_article(&mut self, item: &PocketItem) {
        // Create a file with the new UUID
        let mut article = ArticleHandler::new(item);
        article.save_file("epub", XOCHITL_ROOT).await;

        // Create the content file
        let fname_content = XOCHITL_ROOT.to_string() + "/" + &article.uuid_string() + ".content";
        let content = Content::new("epub");
        Self::write_file(&fname_content, &content);

        // Create the metadata file
        let fname_meta = XOCHITL_ROOT.to_string() + "/" + &article.uuid_string() + ".metadata";
        let metadata = Metadata::new("DocumentType", &article.title(), &self.parent_uuid_string());
        Self::write_file(&fname_meta, &metadata);

        // Add the article to the self.new_items
        self.new_items.insert(UniqID{uuid: article.uuid()},
            item.get_resolved_id().expect("🚨 Expected ID, found None"));
    }


    fn write_file<T>(fname: &str, data: &T) where T: Serialize + std::fmt::Debug {
        let json = serde_json::to_string(&data).expect("🚨 Failed to create json from contents: Content");
        match File::create(fname) {
            Ok(mut fh) => {
                writeln!(fh, "{}", json).expect("🚨 Failed to write contents to file");
            },
            Err(_) => println!("ℹ {fname} already exists, not rewritting."),
        }
    }


    pub fn last_query_ts(&self) -> u64 {
        self.ts_last_query
    }


    pub fn set_last_query_ts(&mut self, ts: u64) {
        self.ts_last_query = ts;
    }


    pub fn read_ids(&self) -> btree_map::IntoValues<UniqID, u64> {
        self.read_items.clone().into_values()
    }


    pub fn consolidate(&mut self) {
        // The first issue needing consolidation is the introduction of the "archive" folder.
        // Since it is a new field, it may not be loaded from the config file. If that's the case,
        // force a proper UUID.
        if self.archive.uuid.is_nil() {
            self.archive = UniqID::new();
        }

        // Go through the list of current items:
        // - If the files is missing, then archive in pocket
        // - If the files exist, but the metadata indicates 'deleted', then archive in pocket
        // - Otherwise it's all good.
        for (uid, apid) in self.current_items.clone() {
            let fname = XOCHITL_ROOT.to_string() + "/" + &utils::uuid_to_string(uid.uuid) + ".metadata";
            let metadata = Metadata::load(&fname);

            if metadata.parent != self.parent_uuid_string() {
                // That is, it is one of:
                // - self.archive_uuid_string()
                // - "trash"
                // - it was moved somewhere else

                // Move item from current list to read list.
                if let Some(val) = self.current_items.remove(&uid) {
                    // Only archive those moved to the archive folder, otherwise assume that the
                    // user is intentionally breaking syncing with Pocket.
                    if metadata.parent == self.archive_uuid_string() {
                        self.read_items.insert(uid.clone(), val);
                        println!("ℹ Moved item with uuid {} into the read_items list", &utils::uuid_to_string(uid.uuid));
                    }
                }
            }
        }

        // Then move the new items to the current items list.
        for (uid, apid) in self.new_items.clone() {
            if let Some(val) = self.new_items.remove(&uid) {
                self.current_items.insert(uid.clone(), val);
                println!("ℹ Moved item with uuid {} into the current_items list", &utils::uuid_to_string(uid.uuid));
            }
        }
    }


    pub fn parent_uuid_string(&self) -> String {
        utils::uuid_to_string(self.folder.uuid)
    }


    pub fn archive_uuid_string(&self) -> String {
        utils::uuid_to_string(self.archive.uuid)
    }


    pub fn clear_read(&mut self) {
        // Move items to the trash in Xochitl, then clear the btreemap
        for (uid, _) in self.read_items.clone() {
            let fname = XOCHITL_ROOT.to_string() + "/" + &utils::uuid_to_string(uid.uuid) + ".metadata";
            let mut metadata = Metadata::load(&fname);

            // "Move" the item to the trash.
            metadata.parent = "trash".to_string();
            // Rewrite the file.
            Self::write_file(&fname, &metadata);

            // Remove the item from the read list.
            if let Some(val) = self.read_items.remove(&uid) {
                self.archived_items.insert(uid.clone(), val);
                println!("ℹ Archived item with uuid {}", &utils::uuid_to_string(uid.uuid));
            }
        }
    }
}


#[derive(Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct UniqID {
    uuid: Uuid,
}

impl UniqID {
    pub fn new() -> Self {
        Self {
            uuid : Uuid::new_v4(),
        }
    }
}


impl Serialize for UniqID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {

        let mut ebuf = Uuid::encode_buffer();
        serializer.serialize_str(self.uuid.hyphenated().encode_lower(&mut ebuf))
    }
}


impl<'de> Deserialize<'de> for UniqID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if s.is_empty() {
            Err(std::io::ErrorKind::InvalidData).map_err(serde::de::Error::custom)
        } else {
            Ok(UniqID{ uuid: Uuid::parse_str(&s).map_err(serde::de::Error::custom)? })
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    deleted: bool,
    #[serde(rename = "lastModified")]
    last_modified: String,
    #[serde(rename = "lastOpenedPage")]
    last_opened_page: u64,
    #[serde(rename = "metadatamodified")]
    metadata_modified: bool,
    modified: bool,
    parent: String,
    pinned: bool,
    synced: bool,
    #[serde(rename = "type")]
    dtype: String,
    version: u64,
    #[serde(rename = "visibleName")]
    visible_name: String,
}

impl Metadata {
    pub fn new(
        dtype: &str,
        name: &str,
        parent: &str
     ) -> Self {
        let start = SystemTime::now();
        let since_epoch = start.duration_since(UNIX_EPOCH).expect("🚨 Time went backwards");
        let last_modified = since_epoch.as_millis();

        Self {
            deleted: false,
            last_modified: last_modified.to_string(),
            last_opened_page: 0,
            metadata_modified: false,
            modified: false,
            parent: parent.to_string(),
            pinned: false,
            synced: false,
            dtype: dtype.to_string(),
            version: 1,
            visible_name: name.to_string(),
        }
    }


    pub fn load(fname: &str) -> Self {
        // Open the file in read-only mode with buffer.
        let fh = File::open(fname).expect(&format!("Couldn't open metadata file {}", fname));
        let reader = BufReader::new(fh);

        // Read the JSON contents of the file as an instance of `User`.
        let metadata = serde_json::from_reader(reader).expect("Couldn't parse JSON from buffer");

        metadata
    }


    pub fn json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}


#[derive(Clone, Debug, Serialize)]
pub struct Content {
    // coverPageNumber: i64,
    // documentMetadata: json,
    // dummyContent: bool,
    #[serde(rename(serialize = "extraMetadata"))]
    extra_meta: serde_json::Value,
    #[serde(rename(serialize = "fileType"))]
    ftype: String,
    #[serde(rename(serialize = "fontName"))]
    font_name: String,
    #[serde(rename(serialize = "lineHeight"))]
    line_height: i64,
    margins: u64,
    #[serde(rename(serialize = "orientation"))]
    orientation: String,
    #[serde(rename(serialize = "pageCount"))]
    page_count: u64,
    // pages: array,
    #[serde(rename(serialize = "textAlignment"))]
    text_alignment: String,
    #[serde(rename(serialize = "textScale"))]
    text_scale: u64,
    transform: serde_json::Value,
}


impl Content {
    pub fn new(ftype: &str) -> Self {
        Self {
            extra_meta: json!({}),
            ftype: ftype.to_string(),
            font_name: "".to_string(),
            line_height: -1,
            margins: 100,
            orientation: "portrait".to_string(),
            page_count: 1, // 1 seems to work well enough
            text_alignment: "left".to_string(),
            text_scale: 1,
            transform: json!({
                "m11": 1,
                "m12": 0,
                "m13": 0,
                "m21": 0,
                "m22": 1,
                "m23": 0,
                "m31": 0,
                "m32": 0,
                "m33": 1
            }),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Version;
    use std::fs;
    use serial_test::serial;
    use std::sync::Once;

    static INIT: Once = Once::new();

    const JSON: &'static str = r#"{
        "folder": "94b8bffc-3e30-4ab8-90d4-64a53140c655",
        "current_items": {
            "2CC4E60A-6212-4DA6-BDD2-FDD713D70943": 9200,
            "4AF52FB0-F787-46AA-84B7-66D0057DBDC5": 42
        },
        "archived_items": {
            "0AE854CA-E195-4029-A861-70D52F71F8E8": 123
        }
    }"#;



    #[test]
    fn build_new() {
        initialize();

        let handler = FSHandler::new();

        assert_eq!(Some(Version::Random), handler.folder.uuid.get_version());
    }

    #[test]
    #[serial]
    fn load_new() {
        initialize();

        let _ = fs::remove_file(CONFIG_FILE);

        let handler = FSHandler::load();
        assert_eq!(Some(Version::Random), handler.folder.uuid.get_version());
        assert!(handler.current_items.is_empty());
        assert!(handler.archived_items.is_empty());
    }


    #[test]
    #[serial]
    fn load_existing() {
        initialize();
        create_test_config();

        let handler = FSHandler::load();

        assert_eq!(Some(Version::Random), handler.folder.uuid.get_version());
        assert_eq!(handler.current_items.len(), 2);
        assert_eq!(handler.archived_items.len(), 1);
    }

    #[test]
    #[serial]
    fn write_config() {
        initialize();
        create_test_config();

        let handler = FSHandler::load();
        let _ = fs::remove_file(CONFIG_FILE);
        handler.save_config();

        let handler = FSHandler::load();
        assert_eq!(Some(Version::Random), handler.folder.uuid.get_version());
        assert_eq!(handler.current_items.len(), 2);
        assert_eq!(handler.archived_items.len(), 1);
    }


    fn initialize() {
        INIT.call_once(|| {
            let _ = fs::remove_dir_all(XOCHITL_ROOT);
            let _ = fs::create_dir_all(XOCHITL_ROOT);
        });
    }


    fn create_test_config() {
        let _ = fs::remove_file(CONFIG_FILE);

        match File::create(CONFIG_FILE) {
            Ok(fh) => {
                let mut buffer = BufWriter::new(fh);
                let _ = buffer.write_all(JSON.as_bytes());
                let _ = buffer.flush();
            },
            Err(e) => {
                panic!("{}", e);
            }
        }
    }
}
