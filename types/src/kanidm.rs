use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub attrs: HashMap<String, Vec<String>>,
}

impl Entry {
    pub fn get_first(&self, attr: &str) -> Option<&str> {
        self.attrs.get(attr)?.first().map(|s| s.as_str())
    }

    pub fn get_all(&self, attr: &str) -> Option<&Vec<String>> {
        self.attrs.get(attr)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub uuid: String,
    pub name: String,
    pub display_name: String,
    pub mail: Option<String>,
    pub groups: Vec<String>,
}

impl TryFrom<Entry> for Person {
    type Error = &'static str;

    fn try_from(entry: Entry) -> Result<Self, Self::Error> {
        Ok(Self {
            uuid: entry.get_first("uuid").ok_or("missing uuid")?.to_string(),
            name: entry.get_first("name").ok_or("missing name")?.to_string(),
            display_name: entry
                .get_first("displayname")
                .unwrap_or("Unknown")
                .to_string(),
            mail: entry.get_first("mail").map(|s| s.to_string()),
            groups: entry
                .get_all("memberof")
                .cloned()
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub uuid: String,
    pub name: String,
    pub members: Vec<String>,
}

impl TryFrom<Entry> for Group {
    type Error = &'static str;

    fn try_from(entry: Entry) -> Result<Self, Self::Error> {
        Ok(Self {
            uuid: entry.get_first("uuid").ok_or("missing uuid")?.to_string(),
            name: entry.get_first("name").ok_or("missing name")?.to_string(),
            members: entry.get_all("member").cloned().unwrap_or_default(),
        })
    }
}
