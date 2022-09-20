use crate::configuration::Settings;
use log::{debug, error, info};
use mangadex_api::{
    types::{Language, MangaFeedSortOrder, OrderDirection},
    v5::schema::{AtHomeServer, ChapterAttributes, ChapterObject},
    MangaDexClient,
};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use resolve_path::PathResolveExt;
use std::fs;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use uuid::Uuid;
use zip::{write::FileOptions, ZipWriter};

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
            .await?;
        let manga_attrs = manga_result.data.attributes;

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
        let manga_path = match manga.directory {
            Some(dir) => dir.resolve().join(&manga_title),
            None => base_path.join(&manga_title),
        };
        fs::create_dir_all(&manga_path)?;

        let feed_result = get_chapters_list(manga.uuid, &client).await;
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

            let filename = get_filename(attrs, &manga_title);
            let page_count = attrs.pages;

            debug!("\"{}\" is {} pages long", &filename, page_count);
            let chapter_path = manga_path.join(&filename);
            if chapter_path.exists() {
                debug!("Chapter {} exists, skipping", &filename);
                continue;
            }

            if let Err(e) = zip_chapter(chapter_uuid, &chapter_path, &client).await {
                error!("Error creating chapter {}: {}", &chapter_path.display(), e);
                continue;
            };
        }
    }

    info!("Finished!");
    Ok(())
}

async fn get_chapters_list(
    uuid: Uuid,
    client: &MangaDexClient,
) -> anyhow::Result<Vec<ChapterObject>> {
    // TODO: This needs a retry
    // TODO: This needs to handle duplicate chapters
    let mut offset: u32 = 0;
    let mut chapters: Vec<ChapterObject> = Vec::new();
    loop {
        let feed_result = client
            .manga()
            .feed()
            .manga_id(&uuid)
            .limit(500_u32)
            .offset(offset)
            .order(MangaFeedSortOrder::Chapter(OrderDirection::Ascending))
            .build()?
            .send()
            .await?;

        let mut result = feed_result?;
        chapters.append(&mut result.data);

        offset += 500;
        if result.total > offset {
            break;
        }
    }

    Ok(chapters)
}

fn get_filename(attrs: &ChapterAttributes, manga_title: &String) -> String {
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
}

async fn get_athomeserver_with_retry(
    uuid: Uuid,
    client: &MangaDexClient,
    reties: u64,
) -> anyhow::Result<AtHomeServer> {
    let mut counter = 0;
    loop {
        let at_home = client
            .at_home()
            .server()
            .chapter_id(&uuid)
            .build()?
            .send()
            .await;
        if let Ok(a) = at_home {
            return Ok(a);
        }
        if counter >= reties {
            at_home?;
        }
        counter += 1;
        thread::sleep(Duration::from_secs(3));
    }
}

async fn zip_chapter(uuid: Uuid, path: &PathBuf, client: &MangaDexClient) -> anyhow::Result<()> {
    info!("Writing chapter {} to {}", &uuid, &path.display());

    let at_home = get_athomeserver_with_retry(uuid, client, 3).await?;

    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let http_client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

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

        info!("Writing page \"{}\"", &page_name);
        zip.start_file(page_name, options)?;
        zip.write_all(&bytes)?;

        page_count += 1;
    }

    zip.finish()?;

    // this is horrible but I get rate limited on short chapters
    if page_count <= 5 {
        thread::sleep(Duration::from_secs(2));
    }

    Ok(())
}
