use aidoku::{
	error::Result,
	prelude::*,
	std::current_date,
	std::net::HttpMethod,
	std::net::Request,
	std::String,
	std::StringRef,
	std::{html::Node, Vec},
	Chapter, DeepLink, Filter, Listing, Manga, MangaContentRating, MangaPageResult, MangaStatus,
	MangaViewer, Page,
};

use crate::helper::*;

pub struct MadaraSiteData {
	pub base_url: String,
	pub lang: String,

	pub source_path: String,
	pub search_path: String,

	pub search_cookies: String,

	pub search_selector: String,
	pub image_selector: String,
	pub genre_selector: String,

	pub status_filter_ongoing: String,
	pub status_filter_completed: String,
	pub status_filter_cancelled: String,
	pub status_filter_on_hold: String,
	pub adult_string: String,
	pub genre_condition: String,
	pub popular: String,
	pub trending: String,

	pub alt_ajax: bool,

	pub viewer: fn(&Node, &Vec<String>) -> MangaViewer,
	pub status: fn(&Node) -> MangaStatus,
	pub nsfw: fn(&Node, &Vec<String>) -> MangaContentRating,
}

impl Default for MadaraSiteData {
	fn default() -> MadaraSiteData {
		MadaraSiteData {
			base_url: String::new(),
			lang: String::from("en"),
			// www.example.com/{source_path}/manga-id/
			source_path: String::from("manga"),
			// www.example.com/{search_path}/?search_query
			search_path: String::from("page"),
			// selector div for search results page
			search_selector: String::from("div.c-tabs-item__content"),
			// cookies to pass for search request
			search_cookies: String::from("wpmanga-adault=1"),
			// div to select images from a chapter
			image_selector: String::from("div.page-break > img"),
			// div to select all the genres
			genre_selector: String::from("div.genres-content > a"),
			// choose between two options for chapter list POST request
			alt_ajax: false,
			// default viewer
			viewer: |_, _| MangaViewer::Scroll,
			status: |html| {
				let status_str = html
					.select("div.post-content_item:contains(Status) div.summary-content")
					.text()
					.read()
					.to_lowercase();
				match status_str.as_str() {
					"ongoing" => MangaStatus::Ongoing,
					"completed" => MangaStatus::Completed,
					"canceled" => MangaStatus::Cancelled,
					"hiatus" => MangaStatus::Hiatus,
					_ => MangaStatus::Unknown,
				}
			},
			nsfw: |html, _| {
				if !html
					.select(".manga-title-badges.adult")
					.text()
					.read()
					.is_empty()
				{
					MangaContentRating::Nsfw
				} else {
					MangaContentRating::Safe
				}
			},
			// Localization stuff
			status_filter_ongoing: String::from("Ongoing"),
			status_filter_completed: String::from("Completed"),
			status_filter_cancelled: String::from("Cancelled"),
			status_filter_on_hold: String::from("On Hold"),
			adult_string: String::from("Adult Content"),
			genre_condition: String::from("Genre Condition"),
			popular: String::from("Popular"),
			trending: String::from("Trending"),
		}
	}
}

pub fn get_manga_list(
	filters: Vec<Filter>,
	page: i32,
	data: MadaraSiteData,
) -> Result<MangaPageResult> {
	let (url, did_search) = get_filtered_url(filters, page, &data);

	if did_search {
		get_search_result(data, url)
	} else {
		get_series_page(data, "_latest_update", page)
	}
}

pub fn get_search_result(data: MadaraSiteData, url: String) -> Result<MangaPageResult> {
	let html = Request::new(url.as_str(), HttpMethod::Get)
		.header("Cookie", &data.search_cookies)
		.html();
	let mut manga: Vec<Manga> = Vec::new();
	let mut has_more = false;

	for item in html.select(data.search_selector.as_str()).array() {
		let obj = item.as_node();

		let id = obj
			.select("a")
			.attr("href")
			.read()
			.replace(&data.base_url.clone(), "")
			.replace(&data.source_path.clone(), "")
			.replace('/', "");
		let title = obj.select("a").attr("title").read();

		let cover = get_image_url(obj.select("img"));

		let genres = obj.select("div.post-content_item div.summary-content a");
		if genres.text().read().to_lowercase().contains("novel") {
			continue;
		}

		manga.push(Manga {
			id,
			cover,
			title,
			author: String::new(),
			artist: String::new(),
			description: String::new(),
			url: String::new(),
			categories: Vec::new(),
			status: MangaStatus::Unknown,
			nsfw: MangaContentRating::Safe,
			viewer: (data.viewer)(&Node::new("<p></p>".as_bytes()), &Vec::new()),
		});
		has_more = true;
	}

	Ok(MangaPageResult { manga, has_more })
}

pub fn get_series_page(data: MadaraSiteData, listing: &str, page: i32) -> Result<MangaPageResult> {
	let url = data.base_url.clone() + "/wp-admin/admin-ajax.php";

	let body_content =  format!("action=madara_load_more&page={}&template=madara-core%2Fcontent%2Fcontent-archive&vars%5Bpaged%5D=1&vars%5Borderby%5D=meta_value_num&vars%5Btemplate%5D=archive&vars%5Bsidebar%5D=full&vars%5Bpost_type%5D=wp-manga&vars%5Bpost_status%5D=publish&vars%5Bmeta_key%5D={}&vars%5Border%5D=desc&vars%5Bmeta_query%5D%5Brelation%5D=OR&vars%5Bmanga_archives_item_layout%5D=big_thumbnail", &page-1, listing);

	let req = Request::new(url.as_str(), HttpMethod::Post)
		.body(body_content.as_bytes())
		.header("Content-Type", "application/x-www-form-urlencoded");

	let html = req.html();

	let mut manga: Vec<Manga> = Vec::new();
	let mut has_more = false;
	for item in html.select("div.page-item-detail").array() {
		let obj = item.as_node();

		let w_novel = obj.select(".web-novel").text().read();
		if !w_novel.is_empty() {
			continue;
		}

		let id = obj
			.select("h3.h5 > a")
			.attr("href")
			.read()
			.replace(&data.base_url.clone(), "")
			.replace(&data.source_path.clone(), "")
			.replace('/', "");

		let title = obj.select("h3.h5 > a").text().read();

		let cover = get_image_url(obj.select("img"));

		manga.push(Manga {
			id,
			cover,
			title,
			author: String::new(),
			artist: String::new(),
			description: String::new(),
			url: String::new(),
			categories: Vec::new(),
			status: MangaStatus::Unknown,
			nsfw: MangaContentRating::Safe,
			viewer: (data.viewer)(&Node::new("<p></p>".as_bytes()), &Vec::new()),
		});
		has_more = true;
	}

	Ok(MangaPageResult { manga, has_more })
}

pub fn get_manga_listing(
	data: MadaraSiteData,
	listing: Listing,
	page: i32,
) -> Result<MangaPageResult> {
	if listing.name == data.popular {
		return get_series_page(data, "_wp_manga_views", page);
	}
	if listing.name == data.trending {
		return get_series_page(data, "_wp_manga_week_views_value", page);
	}
	get_series_page(data, "_latest_update", page)
}

pub fn get_manga_details(manga_id: String, data: MadaraSiteData) -> Result<Manga> {
	let url = data.base_url.clone() + "/" + data.source_path.as_str() + "/" + manga_id.as_str();

	let html = Request::new(url.as_str(), HttpMethod::Get).html();

	let title = html.select("div.post-title h1").text().read();
	let cover = get_image_url(html.select("div.summary_image img"));
	let author = html.select("div.author-content a").text().read();
	let artist = html.select("div.artist-content a").text().read();
	let description = html.select("div.description-summary div p").text().read();

	let mut categories: Vec<String> = Vec::new();
	for item in html.select(data.genre_selector.as_str()).array() {
		categories.push(item.as_node().text().read());
	}

	let status = (data.status)(&html);
	let viewer = (data.viewer)(&html, &categories);
	let nsfw = (data.nsfw)(&html, &categories);

	Ok(Manga {
		id: manga_id,
		cover,
		title,
		author,
		artist,
		description,
		url,
		categories,
		status,
		nsfw,
		viewer,
	})
}

pub fn get_chapter_list(manga_id: String, data: MadaraSiteData) -> Result<Vec<Chapter>> {
	let mut url = data.base_url.clone() + "/wp-admin/admin-ajax.php";
	if data.alt_ajax {
		url = data.base_url.clone()
			+ "/" + data.source_path.as_str()
			+ "/" + manga_id.as_str()
			+ "/ajax/chapters";
	}

	let int_id = get_int_manga_id(manga_id, data.base_url.clone(), data.source_path.clone());
	let body_content = format!("action=manga_get_chapters&manga={}", int_id);

	let req = Request::new(url.as_str(), HttpMethod::Post)
		.body(body_content.as_bytes())
		.header("Content-Type", "application/x-www-form-urlencoded");

	let html = req.html();

	let mut chapters: Vec<Chapter> = Vec::new();
	for item in html.select("li.wp-manga-chapter  ").array() {
		let obj = item.as_node();

		let id = obj
			.select("a")
			.attr("href")
			.read()
			.replace(&(data.base_url.clone() + "/"), "")
			.replace(&(data.source_path.clone() + "/"), "");

		let mut title = String::new();
		let t_tag = obj.select("a").text().read();
		if t_tag.contains('-') {
			title.push_str(t_tag[t_tag.find('-').unwrap() + 1..].trim());
		}

		/*  Chapter number is first occourance of a number in the last element of url
			when split with "/"
			e.g.
			one-piece-color-jk-english/volume-20-showdown-at-alubarna/chapter-177-30-million-vs-81-million/
			will return 177
			parasite-chromatique-french/volume-10/chapitre-062-5/
			will return 62.5
		*/

		let slash_vec = id.as_str().split('/').collect::<Vec<&str>>();

		let dash_split = slash_vec[slash_vec.len() - 2].split('-');
		let dash_vec = dash_split.collect::<Vec<&str>>();

		let mut is_decimal = false;
		let mut chapter = 0.0;
		for obj in dash_vec {
			let mut item = obj.replace('/', "").parse::<f32>().unwrap_or(-1.0);
			if item == -1.0 {
				item = String::from(obj.chars().next().unwrap())
					.parse::<f32>()
					.unwrap_or(-1.0);
			}
			if item != -1.0 {
				if is_decimal {
					chapter += item / 10.0;
					break;
				} else {
					chapter = item;
					is_decimal = true;
				}
			}
		}

		let date_str = obj.select("span.chapter-release-date > i").text().read();
		let mut date_updated = StringRef::from(&date_str)
			.0
			.as_date("MMM d, yyyy", Some("en"), None)
			.unwrap_or(-1.0);
		if date_updated < -1.0 {
			date_updated = StringRef::from(&date_str)
				.0
				.as_date("MMM d, yy", Some("en"), None)
				.unwrap_or(-1.0);
		}
		if date_updated == -1.0 {
			date_updated = current_date();
		}

		let url = obj.select("a").attr("href").read();
		let lang = data.lang.clone();

		chapters.push(Chapter {
			id,
			title,
			volume: -1.0,
			chapter,
			date_updated,
			scanlator: String::new(),
			url,
			lang,
		});
	}
	Ok(chapters)
}

pub fn get_page_list(chapter_id: String, data: MadaraSiteData) -> Result<Vec<Page>> {
	let url = data.base_url.clone() + "/" + data.source_path.as_str() + "/" + chapter_id.as_str();
	let html = Request::new(url.as_str(), HttpMethod::Get).html();

	let mut pages: Vec<Page> = Vec::new();
	for (index, item) in html
		.select(data.image_selector.as_str())
		.array()
		.enumerate()
	{
		pages.push(Page {
			index: index as i32,
			url: get_image_url(item.as_node()),
			base64: String::new(),
			text: String::new(),
		});
	}
	Ok(pages)
}

pub fn modify_image_request(base_url: String, request: Request) {
	request.header("Referer", &base_url);
}

pub fn handle_url(url: String, data: MadaraSiteData) -> Result<DeepLink> {
	let mut manga_id = String::new();
	let parse_url = url.as_str().split('/').collect::<Vec<&str>>();
	if parse_url.len() >= 4 {
		manga_id.push_str(parse_url[4]);
	}
	Ok(DeepLink {
		manga: Some(get_manga_details(manga_id, data)?),
		chapter: None,
	})
}
