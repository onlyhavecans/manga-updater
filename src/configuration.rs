use config::{Config, ConfigError};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub output_directory: String,
    pub mangadex_manga: Vec<MangaDexManga>,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct MangaDexManga {
    pub uuid: Uuid,
    pub directory: Option<String>,
    pub name: Option<String>,
}

impl Settings {
    pub fn new(config_file: &str) -> Result<Self, ConfigError> {
        let builder = Config::builder()
            .add_source(config::File::with_name(config_file))
            .build()?;
        builder.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use uuid::uuid;

    use super::*;

    #[test]
    fn load_config() {
        let c = Settings::new("manga.test.json").unwrap();

        assert_eq!("./test/manga", c.output_directory);
        let _a: OsString = c.output_directory.into();

        let manga1 = MangaDexManga {
            uuid: uuid!("69060a67-1d4e-4110-9d29-838bfd99917f"),
            directory: None,
            name: Some("Bloom Into You".into()),
        };
        let manga2 = MangaDexManga {
            uuid: uuid!("b77668ed-0810-4327-9684-46ca371e370e"),
            directory: None,
            name: None,
        };
        let mangadex_manga = vec![manga1, manga2];
        assert_eq!(mangadex_manga, c.mangadex_manga);
    }
}
