use config::{Config, ConfigError};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub file_format: String,
    pub output_directory: String,
    pub manga: Vec<Uuid>,
}

impl Settings {
    pub fn new(config_file: &str) -> Result<Self, ConfigError> {
        let builder = Config::builder()
            .set_default("file_format", "{name} - v{volume}c{chapter} {chapter_name}")?
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

        assert_eq!("{name} - v{volume}c{chapter} {chapter_name}", c.file_format);
        assert_eq!("/test/directory", c.output_directory);
        let _a: OsString = c.output_directory.into();
        let manga = vec![
            uuid!("b77668ed-0810-4327-9684-46ca371e370e"),
            uuid!("3f1453fb-9dac-4aca-a2ea-69613856c952"),
        ];
        assert_eq!(manga, c.manga);
    }
}
