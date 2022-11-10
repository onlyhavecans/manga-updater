use crate::mangadex;
use url::Url;

pub struct Cbz {
    name: String,
    urls: Vec<Url>,
}

impl Cbz {
    pub fn from(comic: mangadex::Comic) {}

    pub fn write(self) -> anyhow::Result<()> {
        Ok(())
    }
}

fn sanitize_name(s: &str) -> String {
    s.replace([':', '/', '\\'], "")
        // Keep this last to remove duplicate spaces
        .replace("  ", " ")
}

// File_name

// Directory_name

// write_file
