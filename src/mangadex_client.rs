use anyhow::{bail, Result};
use mangadex_api::{
    v5::schema::{AtHomeServer, ChapterObject},
    MangaDexClient,
};
use mangadex_api_types_rust::{MangaFeedSortOrder, OrderDirection};
use std::{thread, time::Duration};
use uuid::Uuid;

pub async fn get_chapters(uuid: Uuid, client: &MangaDexClient) -> Result<Vec<ChapterObject>> {
    // TODO: This needs to handle duplicate chapters
    let mut offset: u32 = 0;
    let mut retry_counter = 0;
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

        // Retry on errors
        if let Err(e) = feed_result {
            if retry_counter > 5 {
                bail!(e)
            }
            retry_counter += 1;
            continue;
        }

        let mut result = feed_result?;
        chapters.append(&mut result.data);

        offset += 500;
        if offset > result.total {
            break;
        }
    }

    Ok(chapters)
}

pub async fn get_athomeserver(
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
