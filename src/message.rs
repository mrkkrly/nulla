use serde::{Deserialize, Serialize};
use crate::event::Event;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClientMessage {
    Event(EventMsg),
    Req(ReqMsg),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventMsg(pub String, pub Event); // ("EVENT", event)

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqMsg(pub String, pub String, pub Filter); // ("REQ", sub_id, filter)


#[derive(Debug, Serialize, Deserialize)]
pub struct Filter {
    #[serde(default)]
    pub kinds: Vec<u32>,
    #[serde(default)]
    pub authors: Vec<String>
}

impl Filter {
    pub fn matches(&self, ev: &Event) -> bool {
        let kind_ok = self.kinds.is_empty() || self.kinds.contains(&ev.kind);
        let author_ok = self.authors.is_empty() || self.authors.contains(&ev.pubkey);
        kind_ok && author_ok
    }
}
