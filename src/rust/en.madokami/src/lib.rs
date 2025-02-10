#![no_std]
extern crate alloc;

use aidoku::{
    error::Result,
    prelude::*,
    std::{
        defaults::defaults_get,
        net::{HttpMethod, Request},
        String, Vec,
    },
    Chapter, DeepLink, Filter, FilterType, Manga, MangaPageResult, MangaStatus, MangaViewer, Page,
};
use base64::{engine::general_purpose, Engine};

mod helper;
use helper::*;

const BASE_URL: &str = "https://manga.madokami.al";

/// Adds HTTP Basic authentication headers to a request if credentials are available.
fn add_auth_to_request(request: Request) -> Result<Request> {
    let username = defaults_get("username")?.as_string()?.read();
    let password = defaults_get("password")?.as_string()?.read();
    if !username.is_empty() && !password.is_empty() {
        let auth = format!(
            "Basic {}",
            general_purpose::STANDARD.encode(format!("{}:{}", username, password))
        );
        Ok(request.header("Authorization", &auth))
    } else {
        Ok(request)
    }
}

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, _page: i32) -> Result<MangaPageResult> {
    // Build URL based on whether we're searching or getting recent.
    let url = if let Some(query) = filters.into_iter()
        .find(|f| matches!(f.kind, FilterType::Title))
        .and_then(|f| f.value.as_string().ok())
        .map(|s| url_encode(&s.read()))
    {
        format!("{}/search?q={}", BASE_URL, query)
    } else {
        format!("{}/recent", BASE_URL)
    };

    let html = add_auth_to_request(Request::new(url.clone(), HttpMethod::Get))?.html()?;
    
    // Select appropriate elements based on page type.
    let selector = if url.ends_with("/recent") {
        "table.mobile-files-table tbody tr td:nth-child(1) a:nth-child(1)"
    } else {
        "div.container table tbody tr td:nth-child(1) a:nth-child(1)"
    };

    let mut mangas = Vec::new();
    for element in html.select(selector).array() {
        if let Ok(node) = element.as_node() {
            let path = node.attr("href").read();
            if !path.ends_with('/') {
                mangas.push(Manga {
                    id: path.clone(),
                    title: extract_manga_title(&path),
                    cover: String::new(),
                    url: format!("{}{}", BASE_URL, path),
                    status: MangaStatus::Unknown,
                    viewer: MangaViewer::Rtl,
                    ..Default::default()
                });
            }
        }
    }

    Ok(MangaPageResult {
        manga: mangas,
        has_more: false,
    })
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
    let html = add_auth_to_request(Request::new(format!("{}{}", BASE_URL, id), HttpMethod::Get))?.html()?;
    let manga_title = extract_manga_title(&id);
    let mut chapters = Vec::new();
    
    for row in html.select("table#index-table > tbody > tr").array() {
        if let Ok(node) = row.as_node() {
            let title = node.select("td:nth-child(1) a").text().read();
            if title.ends_with('/') || title.starts_with('!') {
                continue;
            }

            let base_url = node.select("td:nth-child(6) a").first().attr("href").read();
            let url = match base_url.split("/reader").last() {
                Some(reader_part) => format!("/reader{}", reader_part),
                None => continue,
            };

            let date_updated = node
                .select("td:nth-child(3)")
                .text()
                .as_date("yyyy-MM-dd HH:mm", None, None);
            
            let info = parse_chapter_info(&title, &manga_title);
            
            if let Some((start, end)) = info.chapter_range {
                for ch in (start as i32)..=(end as i32) {
                    chapters.push(Chapter {
                        id: url.clone(),
                        title: format!("Chapter {}", ch),
                        chapter: ch as f32,
                        volume: if info.volume > 0.0 { info.volume } else { -1.0 },
                        date_updated,
                        url: format!("{}{}", BASE_URL, url),
                        ..Default::default()
                    });
                }
            } else {
                chapters.push(Chapter {
                    id: url.clone(),
                    title: url_decode(&title),
                    chapter: if info.chapter > 0.0 { info.chapter } else { -1.0 },
                    volume: if info.volume > 0.0 { info.volume } else { -1.0 },
                    date_updated,
                    url: format!("{}{}", BASE_URL, url),
                    ..Default::default()
                });
            }
        }
    }

    chapters.reverse();
    Ok(chapters)
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
    let mut html = add_auth_to_request(Request::new(format!("{}{}", BASE_URL, id), HttpMethod::Get))?.html()?;
    
    // Get metadata from the current page.
    let mut authors: Vec<String> = html.select("a[itemprop=\"author\"]")
        .array()
        .filter_map(|n| n.as_node().ok().map(|node| node.text().read()))
        .collect();
    let mut genres: Vec<String> = html.select("div.genres a.tag")
        .array()
        .filter_map(|n| n.as_node().ok().map(|node| node.text().read()))
        .collect();
    let mut status = MangaStatus::Unknown;
    let mut cover_url = html.select("div.manga-info img[itemprop=\"image\"]").attr("src").read();
    
    if html.select("span.scanstatus").text().read() == "Yes" {
        status = MangaStatus::Completed;
    }
    
    // If metadata is missing, try using the parent directory.
    if authors.is_empty() || genres.is_empty() || cover_url.is_empty() {
        if let Some(parent_path) = get_parent_path(&id) {
            if let Ok(parent_html) = add_auth_to_request(Request::new(format!("{}{}", BASE_URL, parent_path), HttpMethod::Get))?.html() {
                if cover_url.is_empty() {
                    cover_url = parent_html.select("div.manga-info img[itemprop=\"image\"]").attr("src").read();
                }
                if authors.is_empty() {
                    authors = parent_html.select("a[itemprop=\"author\"]")
                        .array()
                        .filter_map(|n| n.as_node().ok().map(|node| node.text().read()))
                        .collect();
                }
                if genres.is_empty() {
                    genres = parent_html.select("div.genres a.tag")
                        .array()
                        .filter_map(|n| n.as_node().ok().map(|node| node.text().read()))
                        .collect();
                }
                if status == MangaStatus::Unknown && parent_html.select("span.scanstatus").text().read() == "Yes" {
                    status = MangaStatus::Completed;
                }
            }
        }
    }
    
    Ok(Manga {
        id: id.clone(),
        // Use extract_manga_title to derive a clean title from the id.
        title: extract_manga_title(&id),
        author: authors.join(", "),
        cover: cover_url,
        categories: genres,
        status,
        url: format!("{}{}", BASE_URL, id),
        viewer: MangaViewer::Rtl,
        ..Default::default()
    })
}

#[get_page_list]
fn get_page_list(_manga_id: String, chapter_id: String) -> Result<Vec<Page>> {
    let html = add_auth_to_request(Request::new(format!("{}{}", BASE_URL, chapter_id), HttpMethod::Get))?.html()?;
    let reader = html.select("div#reader");
    let path = reader.attr("data-path").read();
    let files = reader.attr("data-files").read();
    
    let mut pages = Vec::new();
    if let Ok(file_list) = aidoku::std::json::parse(files.as_bytes()) {
        if let Ok(array) = file_list.as_array() {
            for (index, file) in array.enumerate() {
                if let Ok(filename) = file.as_string() {
                    pages.push(Page {
                        index: index as i32,
                        url: format!(
                            "{}/reader/image?path={}&file={}",
                            BASE_URL,
                            url_encode(&path),
                            url_encode(&filename.read())
                        ),
                        ..Default::default()
                    });
                }
            }
        }
    }
    
    Ok(pages)
}

#[modify_image_request]
fn modify_image_request(request: Request) {
    if let Ok(request_with_auth) = add_auth_to_request(request) {
        request_with_auth
            .header("Referer", BASE_URL)
            .header("Accept", "image/*");
    }
}

#[handle_url]
fn handle_url(url: String) -> Result<DeepLink> {
    let url = url.replace(BASE_URL, "");
    if url.starts_with("/reader") {
        Ok(DeepLink {
            manga: Some(Manga {
                id: String::from(url.split("/reader").next().unwrap_or_default()),
                ..Default::default()
            }),
            chapter: Some(Chapter {
                id: url,
                ..Default::default()
            }),
        })
    } else {
        Ok(DeepLink {
            manga: Some(Manga {
                id: url,
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
