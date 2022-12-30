#![no_std]

use aidoku::{
	error::Result,
	prelude::*,
	std::net::HttpMethod,
	std::net::Request,
	std::{String, Vec},
	Chapter, DeepLink, Filter, FilterType, Listing, Manga, MangaContentRating, MangaPageResult,
	MangaStatus, MangaViewer, Page,
};
extern crate alloc;
use alloc::string::ToString;
mod helper;

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
	aidoku::prelude::println!("here");
	let base_url = String::from("https://www.tsumino.com");
	let mut sort = String::from("Newest");
	let mut tags: String = String::new();
	let mut i = 0;
	for filter in filters {
		match filter.kind {
			FilterType::Genre => {
				let tpe = 1;
				tags.push_str(&format!(
					"&Tags[{}][Type]={}",
					i.to_string(),
					tpe.to_string()
				));
				tags.push_str(&format!("&Tags[{}][Text]={}", i.to_string(), filter.name));
				match filter.value.as_int().unwrap_or(-1) {
					0 => tags.push_str(&format!("&Tags[{}][Exclude]=true", i.to_string())),
					1 => tags.push_str(&format!("&Tags[{}][Exclude]=false", i.to_string())),
					_ => continue,
				}
				i += 1;
			}
			FilterType::Sort => {
				let value = match filter.value.as_object() {
					Ok(value) => value,
					Err(_) => continue,
				};
				let index = value.get("index").as_int().unwrap_or(0) as i32;
				let option = match index {
					0 => "Newest",
					1 => "Oldest",
					2 => "Alpabetical",
					3 => "Rating",
					4 => "Pages",
					5 => "Views",
					6 => "Random",
					7 => "Comments",
					8 => "Popularity",
					_ => continue,
				};
				sort = String::from(option)
			}
			_ => continue,
		}
	}
	aidoku::prelude::println!("tags: {}", tags);

	let url = String::from(base_url + "/search/operate/");
	let mut parameters = String::new();
	parameters.push_str("PageNumber=");
	parameters.push_str(&helper::urlencode(page.to_string()));
	parameters.push_str("&Sort=");
	parameters.push_str(&helper::urlencode(sort));
	parameters.push_str(&tags);
	aidoku::prelude::println!("url: {}", url);
	aidoku::prelude::println!("parameters: {}", parameters);

	let request = Request::new(&url, HttpMethod::Post)
		.header("User-Agent", "Aidoku")
		.body(format!("{}", parameters));
	let json = request.json()?.as_object()?;
	let data = json.get("data").as_array()?;
	aidoku::prelude::println!("data: {}", data.len());
	let mut manga_arr: Vec<Manga> = Vec::new();
	let total: i32;
	for manga in data {
		aidoku::prelude::println!("a manga");
		let obj = manga.as_object()?;
		let md = obj.get("entry").as_object()?;
		let id = helper::get_id(md.get("id"))?;
		aidoku::prelude::println!("id: {}", id);
		let f = md.get("title");
		aidoku::prelude::println!("executed title thingy: {:?}", f);
		let title = f.as_string()?;
		aidoku::prelude::println!("title len: {}", title.len());
		let string = title.read();
		aidoku::prelude::println!("string: {}", string);

		aidoku::prelude::println!("asnnwkdankjwndka");
		aidoku::prelude::println!("{}", string.is_empty());
		if string.is_empty() {
			aidoku::prelude::println!("empty title");
			continue;
		}
		aidoku::prelude::println!("title: {}", string);
		let cover = md.get("thumbnailUrl").as_string()?.read();
		manga_arr.push(Manga {
			id,
			cover,
			title: string,
			author: String::new(),
			artist: String::new(),
			description: String::new(),
			url: String::new(),
			categories: Vec::new(),
			status: MangaStatus::Completed,
			nsfw: MangaContentRating::Nsfw,
			viewer: MangaViewer::Rtl,
		})
	}
	aidoku::prelude::println!("manga_arr: {}", manga_arr.len());
	total = json.get("pageCount").as_int().unwrap_or(0) as i32;
	aidoku::prelude::println!("total: {}", total);
	aidoku::prelude::println!("page: {}", page);

	Ok(MangaPageResult {
		manga: manga_arr,
		has_more: page < total,
	})
}

#[get_manga_listing]
fn get_manga_listing(_: Listing, _: i32) -> Result<MangaPageResult> {
	todo!()
}

#[get_manga_details]
fn get_manga_details(_: String) -> Result<Manga> {
	todo!()
}

#[get_chapter_list]
fn get_chapter_list(_: String) -> Result<Vec<Chapter>> {
	todo!()
}

#[get_page_list]
fn get_page_list(_: String, _: String) -> Result<Vec<Page>> {
	todo!()
}

#[modify_image_request]
fn modify_image_request(_: Request) {
	todo!()
}

#[handle_url]
fn handle_url(_: String) -> Result<DeepLink> {
	todo!()
}
