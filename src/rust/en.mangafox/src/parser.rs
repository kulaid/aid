use aidoku::{
	error::Result,
	helpers::{substring::Substring, uri::encode_uri},
	prelude::*,
	std::{html::Node, String, Vec},
	Chapter, Filter, FilterType, Manga, MangaContentRating, MangaPageResult, MangaStatus,
	MangaViewer, Page,
};

use crate::unpacker;

extern crate alloc;
use alloc::string::ToString;

pub fn parse_directory(html: Node) -> Result<MangaPageResult> {
	let mut result: Vec<Manga> = Vec::new();
	let has_more: bool = !is_last_page(html.clone());

	for page in html.select("ul.line li").array() {
		let obj = page.as_node().expect("html array not an array of nodes");

		let id = obj
			.select("a")
			.attr("href")
			.read()
			.replace("/manga/", "")
			.replace('/', "");
		let title = obj.select("a").attr("title").read();
		let cover = obj.select("a img").attr("src").read();

		result.push(Manga {
			id,
			cover,
			title,
			status: MangaStatus::Unknown,
			nsfw: MangaContentRating::Safe,
			viewer: MangaViewer::Rtl,
			..Default::default()
		});
	}
	Ok(MangaPageResult {
		manga: result,
		has_more,
	})
}

pub fn parse_manga(obj: Node, id: String) -> Result<Manga> {
	let cover = obj.select(".detail-info-cover-img").attr("src").read();
	let title = obj
		.select("span.detail-info-right-title-font")
		.text()
		.read();
	let author = obj.select("p.detail-info-right-say a").text().read();
	let description = obj.select("p.fullcontent").text().read();

	let url = String::from("https://www.fanfox.net/manga/") + &id;

	let mut viewer = MangaViewer::Rtl;
	let mut nsfw: MangaContentRating = MangaContentRating::Safe;
	let mut categories: Vec<String> = Vec::new();
	obj.select(".detail-info-right-tag-list a")
		.array()
		.for_each(|tag_html| {
			let tag = String::from(
				tag_html
					.as_node()
					.expect("Array of tags wasn't nodes.")
					.text()
					.read()
					.trim(),
			);
			if tag == "Ecchi" || tag == "Mature" || tag == "Smut" || tag == "Adult" {
				nsfw = MangaContentRating::Nsfw;
			}
			if tag == "Webtoons" {
				viewer = MangaViewer::Scroll;
			}
			categories.push(tag);
		});

	let status_str = obj
		.select(".detail-info-right-title-tip")
		.text()
		.read()
		.to_lowercase();
	let status = if status_str.contains("Ongoing") {
		MangaStatus::Ongoing
	} else if status_str.contains("Completed") {
		MangaStatus::Completed
	} else {
		MangaStatus::Unknown
	};

	Ok(Manga {
		id,
		cover,
		title,
		author,
		artist: String::new(),
		description,
		url,
		categories,
		status,
		nsfw,
		viewer,
	})
}

pub fn parse_chapters(obj: Node) -> Result<Vec<Chapter>> {
	let mut chapters: Vec<Chapter> = Vec::new();

	for item in obj.select(".detail-main-list li").array() {
		let obj = item.as_node().expect("");
		let id = obj
			.select("a")
			.attr("href")
			.read()
			.replace("/manga/", "")
			.replace("/1.html", "");

		let url = format!("https://www.fanfox.net/manga/{}", &id);

		// parse title
		let mut title = String::new();
		let title_str = obj.select(".title3").text().read();
		let split = title_str.as_str().split('-');
		let vec = split.collect::<Vec<&str>>();
		if vec.len() > 1 {
			let (_, rest) = vec.split_first().unwrap();
			title = rest.join("-")
		}

		let mut volume = -1.0;
		let mut chapter = -1.0;

		// parse volume and chapter
		let split = id.as_str().split('/');
		let vec = split.collect::<Vec<&str>>();
		for item in vec {
			let f_char = &item.chars().next().unwrap();
			match f_char {
				'v' => {
					volume = String::from(item)
						.trim_start_matches('v')
						.parse::<f32>()
						.unwrap_or(-1.0)
				}
				'c' => {
					chapter = String::from(item)
						.trim_start_matches('c')
						.parse::<f32>()
						.unwrap_or(-1.0)
				}
				_ => continue,
			}
		}

		let date_updated = obj
			.select(".title2")
			.text()
			.0
			.as_date("MMM dd,yyyy", None, None)
			.unwrap_or(-1.0);

		chapters.push(Chapter {
			id,
			title,
			volume,
			chapter,
			date_updated,
			url,
			lang: String::from("en"),
			..Default::default()
		});
	}
	Ok(chapters)
}

pub fn get_page_list(html: Node) -> Result<Vec<Page>> {
	let mut eval_script = String::new();
	for item in html.select("script").array() {
		let script = item.as_node().expect("");
		let body = script.html().read();
		if body.contains("eval(function(p,a,c,k,e,d){") {
			eval_script = body;
		}
	}

	let evaluated = unpacker::unpack(eval_script);

	let page_img_str = evaluated
		.substring_after("var newImgs=[\"//")
		.unwrap()
		.substring_before("\"];var newImginfos=")
		.unwrap()
		.to_string();

	let str_page_arr = page_img_str
		.as_str()
		.split("\",\"//")
		.collect::<Vec<&str>>();

	let mut pages: Vec<Page> = Vec::new();
	for (index, string) in str_page_arr.iter().enumerate() {
		let url = format!("https://{}", string);
		pages.push(Page {
			index: index as i32,
			url: url.to_string(),
			..Default::default()
		});
	}

	Ok(pages)
}

pub fn get_filtered_url(filters: Vec<Filter>, page: i32) -> String {
	let mut is_searching = false;
	let mut search_query = String::new();
	let mut url = String::from("https://fanfox.net");

	let mut genres = String::from("&genres=");
	let mut nogenres = String::from("&nogenres=");

	for filter in filters {
		match filter.kind {
			FilterType::Title => {
				if let Ok(filter_value) = filter.value.as_string() {
					search_query.push_str("&name=");
					search_query.push_str(encode_uri(filter_value.read().to_lowercase()).as_str());
					is_searching = true;
				}
			}
			FilterType::Genre => {
				if let Ok(filter_id) = filter.object.get("id").as_string() {
					match filter.value.as_int().unwrap_or(-1) {
						0 => {
							nogenres.push_str(filter_id.read().as_str());
							nogenres.push(',');
							is_searching = true;
						}
						1 => {
							genres.push_str(filter_id.read().as_str());
							genres.push(',');
							is_searching = true;
						}
						_ => continue,
					}
				}
			}
			FilterType::Select => {
				if filter.name == "Language" {
					search_query.push_str("&type=");
					if filter.value.as_int().unwrap_or(-1) > 0 {
						search_query
							.push_str(&(filter.value.as_int().unwrap_or(-1) as i32).to_string());
						is_searching = true;
					}
				}
				if filter.name == "Rating" {
					search_query.push_str("&rating_method=eq&rating=");
					if filter.value.as_int().unwrap_or(-1) > 0 {
						search_query
							.push_str(&(filter.value.as_int().unwrap_or(-1) as i32).to_string());
						is_searching = true;
					}
				}
				if filter.name == "Completed" {
					search_query.push_str("&st=");
					if filter.value.as_int().unwrap_or(-1) > 0 {
						search_query
							.push_str(&(filter.value.as_int().unwrap_or(-1) as i32).to_string());
						is_searching = true;
					}
				}
			}
			_ => continue,
		}
	}

	if is_searching {
		url.push_str("/search?page=");
		url.push_str(&page.to_string());
		url.push_str(&search_query);
		url.push_str(&genres);
		url.push_str(&nogenres);
	} else {
		url.push_str("/directory/");
		url.push_str(&page.to_string());
		url.push_str(".html?rating")
	}
	encode_uri(url)
}

pub fn parse_incoming_url(url: String) -> String {
	// https://fanfox.net/manga/solo_leveling
	// https://fanfox.net/manga/solo_leveling/c183/1.html#ipg2
	// https://m.fanfox.net/manga/chainsaw_man/
	// https://m.fanfox.net/manga/onepunch_man/vTBD/c178/1.html
	let mut manga_id = url
		.substring_after("/manga/")
		.expect("Could not parse the chapter URL. Make sure the URL for MangaFox is correct.");
	if manga_id.contains('/') {
		manga_id = manga_id.substring_before("/").unwrap();
	}
	manga_id.to_string()
}

pub fn is_last_page(html: Node) -> bool {
	let length = &html.select("div.pager-list-left a").array().len();
	for (index, page) in html.select("div.pager-list-left a").array().enumerate() {
		let page_node = page.as_node().expect("Failed to get page node");
		let href = page_node.attr("href").read();
		if index == length - 1 && href == "javascript:void(0)" {
			return true;
		}
	}
	false
}
