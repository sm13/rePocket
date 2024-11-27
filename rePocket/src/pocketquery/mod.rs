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

use std::str::FromStr;
use strum_macros::{EnumString, FromRepr};
use serde::{Serialize};
use serde_repr::*;


#[derive(Debug, PartialEq, Serialize, EnumString)]
#[serde(rename_all = "lowercase")]
pub enum QState {
    Unread,
    Archive,
    All,
}

#[derive(Debug, PartialEq, Serialize_repr, FromRepr)]
#[repr(u8)]
pub enum QBool {
    No = 0,
    Yes,
}

#[derive(Debug, PartialEq, Serialize, EnumString)]
pub enum QTag {
    Tag(String),
    Untagged,
}

#[derive(Debug, PartialEq, Serialize, EnumString)]
#[serde(rename_all = "lowercase")]
pub enum QContentType {
    Article,
    Video,
    Image,
}

#[derive(Debug, PartialEq, Serialize, EnumString)]
#[serde(rename_all = "lowercase")]
pub enum QSort {
    Newest,
    Oldest,
    Title,
    Site,
}

#[derive(Debug, PartialEq, Serialize, EnumString)]
#[serde(rename_all = "lowercase")]
pub enum QDetailType {
    Simple,
    Complete,
}


#[derive(Debug, Default, Serialize)]
pub struct PocketQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<QState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    favorite: Option<QBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "contentType"))]
    content_type: Option<QContentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort: Option<QSort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "detailType"))]
    detail_type: Option<QDetailType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
    // Although Pocket expects an integer, perhaps we can make it nice, like a DateTime<Local>
    since: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<QBool>,
}


impl PocketQuery {
    pub fn new(
        state: Option<QState>,
        favorite: Option<QBool>,
        tag: Option<String>,
        content_type: Option<QContentType>,
        sort: Option<QSort>,
        detail_type: Option<QDetailType>,
        search: Option<String>,
        domain: Option<String>,
        since: Option<u64>,
        count: Option<u8>,
        offset: Option<u32>,
        total: Option<QBool>,
    ) -> Result<Self, ()> {
        let query = Self {
            state,
            favorite,
            tag,
            content_type,
            sort,
            detail_type,
            search,
            domain,
            since,
            count,
            offset,
            total,
        };

        Ok(query)
    }
}


#[derive(Default)]
pub struct QueryBuilder {
    state: Option<QState>,
    favorite: Option<QBool>,
    tag: Option<String>,
    content_type: Option<QContentType>,
    sort: Option<QSort>,
    detail_type: Option<QDetailType>,
    search: Option<String>,
    domain: Option<String>,
    since: Option<u64>,
    count: Option<u8>,
    offset: Option<u32>,
    total: Option<QBool>,
}

impl QueryBuilder {
    pub fn set_state(mut self, state: &str) -> Self {
        self.state = Some(QState::from_str(state).unwrap());

        self
    }

    pub fn set_favorite(mut self, favorite: u8) -> Self {
        self.favorite = QBool::from_repr(favorite);

        self
    }

    #[allow(dead_code)]
    pub fn set_tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());

        self
    }

    #[allow(dead_code)]
    pub fn set_content_type(mut self, content_type: &str) -> Self {
        self.content_type = Some(QContentType::from_str(content_type).unwrap());

        self
    }

    pub fn set_sort(mut self, sort: &str) -> Self {
        self.sort = Some(QSort::from_str(sort).unwrap());

        self
    }

    pub fn set_detail_type(mut self, detail_type: &str) -> Self {
        self.detail_type = Some(QDetailType::from_str(detail_type).unwrap());

        self
    }

    #[allow(dead_code)]
    pub fn set_search(mut self, search: &str) -> Self {
        self.search = Some(search.into());

        self
    }

    #[allow(dead_code)]
    pub fn set_domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.into());

        self
    }

    // Perhaps timestamp should be of a type similar to whatever the crate chrono uses.
    pub fn set_since(mut self, timestamp: u64) -> Self {
        self.since = Some(timestamp);

        self
    }

    // Truncates at 30, as per the Pocket API
    pub fn set_count(mut self, count: u8) -> Self {
        self.count = Some(std::cmp::min(count, 30));

        self
    }

    pub fn set_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);

        self
    }

    pub fn set_total(mut self, total: u8) -> Self {
        self.total = QBool::from_repr(total);

        self
    }

    pub fn build(self) -> Result<PocketQuery, ()> {
        PocketQuery::new(
            self.state,
            self.favorite,
            self.tag,
            self.content_type,
            self.sort,
            self.detail_type,
            self.search,
            self.domain,
            self.since,
            self.count,
            self.offset,
            self.total,
        )   
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_using_default_builder() {
        let query = QueryBuilder::default().build().unwrap();
        assert_eq!(query.state, None);
        assert_eq!(query.favorite, None);
        assert_eq!(query.tag, None);
        assert_eq!(query.content_type, None);
        assert_eq!(query.sort, None);
        assert_eq!(query.detail_type, None);
        assert_eq!(query.search, None);
        assert_eq!(query.domain, None);
        assert_eq!(query.since, None);
        assert_eq!(query.count, None);
        assert_eq!(query.offset, None);
        assert_eq!(query.total, None);

    }

    #[test]
    fn build_query_using_builder() {

        let query = QueryBuilder::default()
            .set_state("Unread")
            .set_favorite(0)
            .set_tag("rust")
            .set_content_type("Article")
            .set_sort("Newest")
            .set_detail_type("Simple")
            .set_search("learn")
            .set_domain(".com")
            //.set_since("");
            .set_count(10)
            .set_offset(0)
            .set_total(1)
            .build()
            .expect("Failed to build query");

        assert_eq!(query.state, Some(QState::Unread));
        assert_eq!(query.favorite, Some(QBool::No));
        assert_eq!(query.tag, Some("rust".to_string()));
        assert_eq!(query.content_type, Some(QContentType::Article));
        assert_eq!(query.sort, Some(QSort::Newest));
        assert_eq!(query.detail_type, Some(QDetailType::Simple));
        assert_eq!(query.search, Some("learn".to_string()));
        assert_eq!(query.domain, Some(".com".to_string()));
        //assert_eq!(query.since.unwrap(), );
        assert_eq!(query.count, Some(10));
        assert_eq!(query.offset, Some(0));
        assert_eq!(query.total, Some(QBool::Yes));
    }
}
