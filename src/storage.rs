use std::collections::HashMap;
use url::Url;

pub struct Storage {
    data: HashMap<String, Links>
}

struct Links {
    pending: Vec<Url>,
    archived: Vec<Url>,
}

impl Storage {

    pub fn new() -> Storage {
        let mut data = HashMap::<String, Links>::new();
        data.insert("pipi".to_string(), Links {
            pending: vec![Url::parse("http://google.com").unwrap()],
            archived: vec![]
        });
        Storage {
            data
        }
    }

    pub async fn add(&mut self, id: &str, link: Url) {
        match self.data.get_mut(id) {
            Some(links) =>
                if !links.pending.contains(&link) {
                    links.pending.push(link);
                },
            None => {}

        };
    }

    pub async fn archive(&mut self, id: &str, link: Url) {
        match self.data.get_mut(id) {
            Some(links) =>
                if !links.pending.contains(&link) {
                    links.pending.retain(|el| el == &link);
                    links.archived.push(link);
                },
            None => {}

        };
    }

    pub async fn pending_list(&self, id: String) -> &Vec<Url> {
        &self.data.get(&id).unwrap().pending
    }

    pub async fn archived_list(&self) -> Vec<Url> {
        unimplemented!();
    }
}