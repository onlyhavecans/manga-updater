use clap::Parser;
use env_logger::{Builder, Env, Target};
use log::{debug, error, info};
use manga_updater::configuration::Settings;
use mangadex_api::{types::Language, MangaDexClient};
use std::process;

#[derive(clap::Parser)]
struct Args {
    #[clap(short, long, default_value = "manga.json")]
    config_file: String,
}

#[tokio::main]
async fn main() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("debug"));
    builder.target(Target::Stdout);
    builder.init();

    let args = Args::parse();

    let settings = Settings::new(&args.config_file);
    if let Err(e) = settings {
        error!("Configuration error: {}", e);
        process::exit(1);
    }
    let s = settings.unwrap();

    if let Err(e) = run(s).await {
        error!("Application error: {}", e);
        process::exit(1);
    }
}

async fn run(settings: Settings) -> anyhow::Result<()> {
    debug!("Output Directory: {}", settings.output_directory);
    debug!("Manga {:?}", settings.manga);

    let client = MangaDexClient::default();

    for uuid in settings.manga {
        let manga_result = client.manga().get().manga_id(&uuid).build()?.send().await?;
        let manga_attrs = manga_result.data.attributes;
        let name = manga_attrs
            .title
            .get(&mangadex_api::types::Language::English)
            .unwrap();
        info!("Manga: {}", name);

        let feed_result = client
            .manga()
            .feed()
            .manga_id(&uuid)
            .build()?
            .send()
            .await?;

        if let Err(e) = feed_result {
            error!("Unable to retrieve {}: {}", uuid, e);
            continue;
        }
        let manga_chapters = feed_result?.data;
        let english_chapters = manga_chapters
            .iter()
            .filter(|c| c.attributes.translated_language == Language::English);
        for chapter in english_chapters {
            let attrs = &chapter.attributes;
            if attrs.translated_language != Language::English {
                continue;
            }

            let title = if attrs.title.is_empty() {
                "".into()
            } else {
                format!("- {}", &attrs.title)
            };
            let volume = match &attrs.volume {
                Some(v) => v,
                None => "U",
            };
            let chapter = match &attrs.chapter {
                Some(c) => c,
                None => "U",
            };
            let pages = attrs.pages;
            let filename = format!("{} - v{}c{}{}.cbz", name, volume, chapter, title);
            info!("\"{}\" is {} pages long", filename, pages);
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn clap_test() {
        use clap::CommandFactory;
        Args::command().debug_assert()
    }
}
