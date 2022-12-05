use aidoku::{
	error::Result, prelude::*, std::defaults::defaults_get, std::html::Node, std::String, std::Vec,
	Chapter, Filter, FilterType, Manga, MangaContentRating, MangaStatus, MangaViewer, Page,
};

use crate::crypto::batojs_decrypt;
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

	for i in obj.select(".attr-item").array() {
		let item = i.as_node();
		if item.select("b").text().read().contains("Author") {
			author = item.select("span").text().read();
		}
		if item.select("b").text().read().contains("Artist") {
			artist = item.select("span").text().read();
		}
		if item.select("b").text().read().contains("Original") {
			status_str = item.select("span").text().read();
		}
		if item.select("b").text().read().contains("Genres") {
			let gen_str = item.select("span").text().read();
			let split = gen_str.as_str().split(',');
			let vec = split.collect::<Vec<&str>>();
			for item in vec {
				categories.push(String::from(item.trim()));
			}
		}
	}

	let url = format!("https://bato.to/series/{}", &id);

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
		viewer: MangaViewer::Scroll,
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
		let _time_str = chapter_node.select(".extra i.ps-3").text().read();

		let chapter = String::from(name.trim()).parse::<f32>().unwrap_or(-1.0);
		let mut url = String::from("https://bato.to/chapter/");
		url.push_str(&id);

		chapters.push(Chapter {
			id,
			title,
			volume: -1.0,
			chapter,
			date_updated: -1.0,
			scanlator: String::new(),
			url,
			lang: String::from("en"),
		});
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
		let t = tkn_str.replace('[', "").replace(']', "");
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
	let mut url = String::from("https://bato.to");
	let mut search = false;

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

// HELPER FUNCTIONS

pub fn i32_to_string(mut integer: i32) -> String {
	if integer == 0 {
		return String::from("0");
	}
	let mut string = String::with_capacity(11);
	let pos = if integer < 0 {
		string.insert(0, '-');
		1
	} else {
		0
	};
	while integer != 0 {
		let mut digit = integer % 10;
		if pos == 1 {
			digit *= -1;
		}
		string.insert(pos, char::from_u32((digit as u32) + ('0' as u32)).unwrap());
		integer /= 10;
	}
	string
}

pub fn urlencode(string: String) -> String {
	let mut result: Vec<u8> = Vec::with_capacity(string.len() * 3);
	let hex = "0123456789abcdef".as_bytes();
	let bytes = string.as_bytes();

	for byte in bytes {
		let curr = *byte;
		if (b'a'..=b'z').contains(&curr)
			|| (b'A'..=b'Z').contains(&curr)
			|| (b'0'..=b'9').contains(&curr)
		{
			result.push(curr);
		} else {
			result.push(b'%');
			result.push(hex[curr as usize >> 4]);
			result.push(hex[curr as usize & 15]);
		}
	}

	String::from_utf8(result).unwrap_or_default()
}
