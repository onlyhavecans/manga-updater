use url::Url;
use uuid::Uuid;

pub struct Comic {
    name: String,
    uuid: Uuid,
    chapters: Vec<Chapter>,
}

pub struct Chapter {
    name: String,
    pages: Vec<Page>,
}

pub struct Page {
    url: Url,
}
