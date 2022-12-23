use aidoku::{
	error::Result,
	prelude::*,
	std::{current_date, defaults::defaults_get, html::Node, String, Vec},
	Chapter, Filter, FilterType, Manga, MangaContentRating, MangaStatus, MangaViewer, Page,
};

use crate::crypto::batojs_decrypt;
use crate::helper::{i32_to_string, lang_encoder, urlencode};
use crate::substring::Substring;
// use alloc::string::String;

pub fn parse_listing(html: Node, result: &mut Vec<Manga>) {
	for page in html.select(".col.item").array() {
		let obj = page.as_node();

		let id = obj
			.select(".item-cover")
			.attr("href")
			.read()
			.replace("/series/", "");
		let title = obj.select(".item-title").text().read();
		let img = obj.select(".item-cover img").attr("src").read();

		result.push(Manga {
			id,
			cover: img,
			title,
			author: String::new(),
			artist: String::new(),
			description: String::new(),
			url: String::new(),
			categories: Vec::new(),
			status: MangaStatus::Unknown,
			nsfw: MangaContentRating::Safe,
			viewer: MangaViewer::Default,
		});
	}
}

pub fn parse_search(html: Node, result: &mut Vec<Manga>) {
	for page in html.select("#series-list .item").array() {
		let obj = page.as_node();

		let id = obj
			.select(".item-cover")
			.attr("href")
			.read()
			.replace("/series/", "");
		let title = obj.select(".item-title").text().read();
		let img = obj.select("img").attr("src").read();

		if !id.is_empty() && !title.is_empty() && !img.is_empty() {
			result.push(Manga {
				id,
				cover: img,
				title,
				author: String::new(),
				artist: String::new(),
				description: String::new(),
				url: String::new(),
				categories: Vec::new(),
				status: MangaStatus::Unknown,
				nsfw: MangaContentRating::Safe,
				viewer: MangaViewer::Default,
			});
		}
	}
}

pub fn parse_manga(obj: Node, id: String) -> Result<Manga> {
	let title = obj.select(".item-title").text().read();
	let cover = obj.select(".shadow-6").attr("src").read();
	let description = obj.select(".limit-html").text().read();

	let mut author = String::new();
	let mut artist = String::new();
	let mut status_str = String::new();
	let mut categories: Vec<String> = Vec::new();
	let mut viewer = MangaViewer::Scroll;

	let mut is_webtoon = false;

	for i in obj.select(".attr-item").array() {
		let item = i.as_node();
		let label_title = item.select("b").text().read();
		if label_title.contains("Author") {
			author = item.select("span").text().read();
		}
		if label_title.contains("Artist") {
			artist = item.select("span").text().read();
		}
		if label_title.contains("Original") {
			status_str = item.select("span").text().read();
		}
		if label_title.contains("Genre") {
			for genre_span in item.select("span span").array() {
				let genre_string = genre_span.as_node();
				categories.push(genre_string.text().read());
				if genre_string.text().read() == "Webtoon" {
					is_webtoon = true;
				}
			}
		}
		if label_title.contains("Read direction") {
			let view_string = item.select("span").text().read();
			if view_string.contains("Left to Right") || view_string.contains("Right to Left") {
				viewer = MangaViewer::Rtl;
			}
		}
	}
	// Webtoon titles may be improperly set to Rtl or Ltr by the source.
	if is_webtoon {
		viewer = MangaViewer::Scroll;
	}

	let mut url = String::new();
	if let Ok(url_str) = defaults_get("sourceURL").as_string() {
		url.push_str(url_str.read().as_str());
		url.push_str("/series/");
		url.push_str(&id);
	}

	let status = if status_str.contains("Ongoing") {
		MangaStatus::Ongoing
	} else if status_str.contains("Completed") {
		MangaStatus::Completed
	} else if status_str.contains("Hiatus") {
		MangaStatus::Hiatus
	} else if status_str.contains("Cancelled") {
		MangaStatus::Cancelled
	} else {
		MangaStatus::Unknown
	};

	let mut nsfw = MangaContentRating::Safe;
	if !obj
		.select(".alert.alert-warning span b")
		.text()
		.read()
		.is_empty()
	{
		nsfw = MangaContentRating::Nsfw;
	}

	Ok(Manga {
		id,
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

pub fn get_chaper_list(obj: Node) -> Result<Vec<Chapter>> {
	let mut chapters: Vec<Chapter> = Vec::new();
	for item in obj.select(".item").array() {
		let chapter_node = item.as_node();
		let id = chapter_node
			.select("a")
			.attr("href")
			.read()
			.replace("/chapter/", "");
		let title = chapter_node.select(".chapt span").text().read();
		let name = chapter_node
			.select("a b")
			.text()
			.read()
			.replace("Chapter", "");

		let scanlator = chapter_node.select("div.extra a.ps-3 span").text().read();
		let time_str = chapter_node.select(".extra i.ps-3").text().read();
		let mut date_updated = current_date();
		// if date is in minutes/hours, then the date is current_date(), no higher
		// denomination that days exist.
		if time_str.contains("days") {
			let date_num = time_str.split(' ').collect::<Vec<&str>>()[0]
				.parse::<f64>()
				.unwrap();
			date_updated -= date_num * 24.0 * 60.0 * 60.0;
		}

		let mut lang = String::from("en");
		for i in obj.select(".attr-item").array() {
			let item = i.as_node();
			let label_title = item.select("b").text().read();
			if label_title.contains("Translated") {
				let lang_str = item.select("span").text().read();
				lang = lang_encoder(lang_str);
			}
		}

		let chapter = String::from(name.trim()).parse::<f32>().unwrap_or(-1.0);
		if let Ok(url_str) = defaults_get("sourceURL").as_string() {
			let mut url = url_str.read();
			url.push_str("/chapter/");
			url.push_str(&id);

			chapters.push(Chapter {
				id,
				title,
				volume: -1.0,
				chapter,
				date_updated,
				scanlator,
				url,
				lang,
			});
		}
	}
	Ok(chapters)
}

pub fn get_page_list(obj: Node) -> Result<Vec<Page>> {
	let mut pages: Vec<Page> = Vec::new();

	for item in obj.select("body script").array() {
		let script = item.as_node();
		let script_text = script.html().read();
		if !script_text.contains("your_email") {
			continue;
		}

		let bato_js;
		match script_text.substring_after_last("const batoPass = ") {
			Some(v) => match v.substring_before(";") {
				Some(w) => bato_js = w,
				None => panic!(),
			},
			None => panic!(),
		}
		let img_str;
		match script_text.substring_after_last("const imgHttpLis = [\"") {
			Some(v) => match v.substring_before("\"];") {
				Some(w) => img_str = w,
				None => panic!(),
			},
			None => panic!(),
		}
		let server_token;
		match script_text.substring_after_last("batoWord = \"") {
			Some(v) => match v.substring_before("\";") {
				Some(w) => server_token = w,
				None => panic!(),
			},
			None => panic!(),
		}
		let img_arr = img_str.split("\",\"").collect::<Vec<&str>>();
		let tkn_str = batojs_decrypt(String::from(server_token), String::from(bato_js));
		let t = tkn_str.replace(['[', ']'], "");
		let tkn_arr = t.split(',').collect::<Vec<&str>>();

		for (index, _item) in img_arr.iter().enumerate() {
			let ind = index as i32;
			let url = format!("{}?{}", _item, tkn_arr[index]);
			pages.push(Page {
				index: ind,
				url,
				base64: String::new(),
				text: String::new(),
			});
		}
	}
	Ok(pages)
}

pub fn get_filtered_url(filters: Vec<Filter>, page: i32) -> (String, bool) {
	let mut url = String::new();
	let mut search = false;

	if let Ok(url_str) = defaults_get("sourceURL").as_string() {
		url.push_str(url_str.read().as_str());
	}
	for filter in filters {
		match filter.kind {
			FilterType::Title => {
				if let Ok(filter_value) = filter.value.as_string() {
					url.push_str("/search?word=");
					url.push_str(urlencode(filter_value.read().to_lowercase()).as_str());
					url.push_str("&page=");
					url.push_str(&i32_to_string(page));
					search = true;
					break;
				}
			}
			_ => continue,
		}
	}
	if !search {
		get_list_url(&mut url, "title.az", page);
	}
	(url, search)
}

pub fn get_list_url(url: &mut String, sort_type: &str, page: i32) {
	if let Ok(languages) = defaults_get("languages").as_array() {
		url.push_str("/browse?langs=");
		for lang in languages {
			if let Ok(lang) = lang.as_string() {
				url.push_str(&lang.read());
				url.push(',');
			}
		}
	}
	url.push_str("&sort=");
	url.push_str(sort_type);
	url.push_str("&page=");
	url.push_str(&i32_to_string(page));
}

pub fn parse_incoming_url(url: String) -> String {
	//bato.to/series/72873/who-made-me-a-princess-official

	let split = url.as_str().split('/');
	let vec = split.collect::<Vec<&str>>();
	let mut manga_id = String::new();

	if url.contains("/chapters/") {
	} else {
		manga_id.push_str(vec[vec.len() - 2]);
		manga_id.push('/');
		manga_id.push_str(vec[vec.len() - 1]);
	}

	manga_id
}
