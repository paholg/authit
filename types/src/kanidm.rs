use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::err;

#[derive(Deserialize)]
pub struct RawPerson {
    attrs: PersonAttrs,
}

#[derive(Deserialize)]
struct PersonAttrs {
    uuid: Vec<Uuid>,
    name: Vec<String>,
    displayname: Vec<String>,
    mail: Vec<String>,
    memberof: Vec<String>,
}

#[derive(Deserialize)]
pub struct RawGroup {
    attrs: GroupAttrs,
}

#[derive(Deserialize)]
struct GroupAttrs {
    uuid: Vec<Uuid>,
    name: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, Deserialize, Serialize)]
pub struct Person {
    pub uuid: Uuid,
    pub name: String,
    pub display_name: String,
    pub email_addresses: Vec<String>,
    pub groups: Vec<String>,
}

impl std::cmp::PartialOrd for Person {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.display_name.partial_cmp(&other.display_name)
    }
}

impl TryFrom<RawPerson> for Person {
    type Error = crate::Error;

    fn try_from(value: RawPerson) -> Result<Self, Self::Error> {
        let attrs = value.attrs;
        Ok(Self {
            uuid: attrs
                .uuid
                .into_iter()
                .next()
                .ok_or_else(|| err!("missing uuid for person"))?,
            name: attrs
                .name
                .into_iter()
                .next()
                .ok_or_else(|| err!("missing name for person"))?,
            display_name: attrs
                .displayname
                .into_iter()
                .next()
                .ok_or_else(|| err!("missing displayname for person"))?,
            email_addresses: attrs.mail,
            groups: attrs.memberof,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, Deserialize, Serialize)]
pub struct Group {
    pub uuid: Uuid,
    pub name: String,
}

impl std::cmp::PartialOrd for Group {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl TryFrom<RawGroup> for Group {
    type Error = crate::Error;

    fn try_from(value: RawGroup) -> Result<Self, Self::Error> {
        let attrs = value.attrs;
        Ok(Self {
            uuid: attrs
                .uuid
                .into_iter()
                .next()
                .ok_or_else(|| err!("missing uuid for group"))?,
            name: attrs
                .name
                .into_iter()
                .next()
                .ok_or_else(|| err!("missing name for group"))?,
        })
    }
}
