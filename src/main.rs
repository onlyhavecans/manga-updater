use clap::Parser;
use env_logger::{Builder, Env, Target};
use log::{debug, error, info};
use manga_updater::configuration::Settings;
use mangadex_api::{types::Language, v5::schema::ChapterAttributes, MangaDexClient};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process,
};
use uuid::Uuid;
use zip::{write::FileOptions, ZipWriter};

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
    let base_path = Path::new(&settings.output_directory);
    fs::create_dir_all(base_path)?;

    debug!("Manga {:?}", settings.manga);

    let client = MangaDexClient::default();

    for uuid in settings.manga {
        let manga_result = client.manga().get().manga_id(&uuid).build()?.send().await?;
        let manga_attrs = manga_result.data.attributes;
        let manga_title = manga_attrs
            .title
            .get(&mangadex_api::types::Language::English)
            .unwrap();
        info!("Manga: {}", manga_title);
        let manga_path = base_path.join(manga_title);
        fs::create_dir_all(&manga_path)?;

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
            let chapter_uuid = chapter.id;
            let attrs = &chapter.attributes;
            if attrs.translated_language != Language::English {
                continue;
            }
            let filename = get_filename(attrs, manga_title);
            let page_count = attrs.pages;
            debug!("\"{}\" is {} pages long", &filename, page_count);
            let chapter_path = manga_path.join(&filename);
            if chapter_path.exists() {
                info!("Chapter {} exists, Skipping", &filename);
                continue;
            }

            zip_chapter(chapter_uuid, &chapter_path, &client).await?;
        }
    }

    Ok(())
}

fn get_filename(attrs: &ChapterAttributes, manga_title: &String) -> String {
    let chapter_title = if attrs.title.is_empty() {
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

    format!(
        "{} - v{}c{}{}.cbz",
        manga_title, volume, chapter, chapter_title
    )
}

async fn zip_chapter(uuid: Uuid, path: &PathBuf, client: &MangaDexClient) -> anyhow::Result<()> {
    info!("Writing chapter {} to {}", &uuid, &path.display());

    let at_home = client
        .at_home()
        .server()
        .chapter_id(&uuid)
        .build()?
        .send()
        .await?;

    let http_client = reqwest::Client::new();

    debug!("Creating {}", &path.display());
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    let mut page_count = 1;
    let page_filenames = at_home.chapter.data;
    for filename in page_filenames {
        debug!("Getting page #{}: {}", &page_count, &filename);
        let page_url = at_home.base_url.join(&format!(
            "/{quality_mode}/{chapter_hash}/{page_filename}",
            quality_mode = "data",
            chapter_hash = at_home.chapter.hash,
            page_filename = filename
        ))?;

        let page_ext = Path::new(&filename).extension().unwrap();
        let page_name = format!("page {:>03}.{}", page_count, page_ext.to_string_lossy());
        let res = http_client.get(page_url).send().await?;
        // The data should be streamed rather than downloading the data all at once.
        let bytes = res.bytes().await?;

        debug!("Writing page {}", &page_name);
        zip.start_file(page_name, options)?;
        zip.write_all(&bytes)?;

        page_count += 1;
    }

    zip.finish()?;
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
