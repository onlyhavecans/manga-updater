use crate::mangadex_client::{get_athomeserver, get_chapters};
use crate::Settings;
use async_zip::write::{EntryOptions, ZipFileWriter};
use async_zip::Compression;
use log::{debug, error, info};
use mangadex_api::types::Language;
use mangadex_api::v5::schema::{ChapterAttributes, ChapterObject};
use mangadex_api::MangaDexClient;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use resolve_path::PathResolveExt;
use std::fs;
use std::{
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use tokio::fs::File;
use uuid::Uuid;

pub async fn run(settings: Settings) -> anyhow::Result<()> {
    info!("Output Directory: {}", settings.output_directory);
    let base_path = settings.output_directory.resolve();
    fs::create_dir_all(&base_path)?;
    let manga_list = settings.mangadex_manga;

    debug!("Manga {:?}", manga_list);

    let client = MangaDexClient::default();

    for manga in manga_list {
        let manga_result = client
            .manga()
            .get()
            .manga_id(&manga.uuid)
            .build()?
            .send()
            .await;
        if let Err(e) = manga_result {
            error!(
                "Unable to retrieve manga uuid {}: {}, skipping",
                manga.uuid, e
            );
            continue;
        }
        let manga_attrs = manga_result?.data.attributes;

        // Use title from config or english title
        let manga_title = match manga.name {
            Some(name) => name,
            None => manga_attrs
                .title
                .get(&mangadex_api::types::Language::English)
                .unwrap()
                .into(),
        };

        info!("Checking Manga: {}", manga_title);

        // If override path on manga's config use it
        let manga_path = base_path.join(&manga_title);
        fs::create_dir_all(&manga_path)?;

        let feed_result = get_chapters(manga.uuid, &client).await;
        if let Err(e) = feed_result {
            error!(
                "Unable to retrieve feed for {}: {}, skipping",
                manga.uuid, e
            );
            continue;
        }

        let manga_chapters: Vec<ChapterObject> = feed_result?;
        let english_chapters = manga_chapters
            .iter()
            .filter(|c| c.attributes.translated_language == Language::English);

        for chapter in english_chapters {
            let chapter_uuid = chapter.id;
            let attrs = &chapter.attributes;

            let filename = generate_filename(attrs, &manga_title);
            let page_count = attrs.pages;

            debug!("\"{}\" is {} pages long", &filename, page_count);
            let chapter_path = manga_path.join(&filename);
            if chapter_path.exists() {
                debug!("Chapter {} exists, skipping", &filename);
                continue;
            }

            if let Err(e) = zip_chapter(chapter_uuid, &chapter_path, &client).await {
                error!("Error creating chapter {}: {}", &chapter_path.display(), e);
                // clean up incomplete files, discard errors
                let _ = fs::remove_file(&chapter_path);
                continue;
            };
        }
    }

    info!("Finished!");
    Ok(())
}

fn generate_filename(attrs: &ChapterAttributes, manga_title: &String) -> String {
    let chapter_title = if attrs.title.is_empty() {
        "".into()
    } else {
        format!(" - {}", &attrs.title)
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
    .replace(':', "")
    .replace('/', "")
    .replace('\\', "")
}

async fn zip_chapter(uuid: Uuid, path: &PathBuf, client: &MangaDexClient) -> anyhow::Result<()> {
    info!("Writing chapter {} to {}", &uuid, &path.display());

    let at_home = get_athomeserver(uuid, client, 3).await?;

    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let http_client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    debug!("Creating {}", &path.display());
    let mut file = File::create(path).await?;
    let mut zip = ZipFileWriter::new(&mut file);

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

        let page_ext = match Path::new(&filename).extension() {
            Some(ext) => ext,
            None => {
                info!("Skipping page with no extension: {}", &filename);
                continue;
            }
        };

        let page_name = format!("page {:>03}.{}", page_count, page_ext.to_string_lossy());
        let res = http_client.get(page_url).send().await?;
        // The data should be streamed rather than downloading the data all at once.
        let bytes = res.bytes().await?;

        info!("Writing page \"{}\"", &page_name);
        let options = EntryOptions::new(page_name, Compression::Deflate);
        zip.write_entry_whole(options, &bytes).await?;

        page_count += 1;
    }

    zip.close().await?;

    // I get rate limited on short chapters
    if page_count <= 5 {
        thread::sleep(Duration::from_secs(2));
    }

    Ok(())
}
