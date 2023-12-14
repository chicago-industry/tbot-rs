// #![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]
// #![allow(unused_mut)]

use chrono::NaiveTime;
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{Element, ElementRef, Html, Selector};
use std::{error::Error, fmt, io};

pub mod cinema;
pub mod movie;
pub mod showtime;

type CustomError = Box<dyn Error + Send + Sync>;
type CustomResult<T> = Result<T, CustomError>;

// <div class="step" data-id="1">
//     <div class="aside">
//         <div class="place-name">Сатурн</div>
//         <div class="contact">
//             <p>Снежная ул., д. 18</p>
//             <div class="metro">
//                 <span style="color: #EF8532;">●</span>
//                 Свиблово
//             </div>
//         </div>
//     </div>
//     <div class="content">
//         <div class="schedule-item">
//             <div class="title">
//                 По щучьему велению <small>2023 / 115 мин / Россия / 6+</small>
//             </div>
//             <div class="list">
//                 <a href="javascript:ticketManager.richSession(96619320)" class="subitem">
//                     <span class="time">13:00</span>
//                     <span class="badge">2D</span>
//                     <span class="price">200 P</span>
//                 </a>
//                 <a href="javascript:ticketManager.richSession(96546614)" class="subitem">
//                     <span class="time">19:25</span>
//                     <span class="badge">2D</span>
//                     <span class="price">300 P</span>
//                 </a>
//             </div>
//         </div>
//         ...
//         ...
//         ...
//         <div class="schedule-item">
//             <div class="title">
//                 Следующая жертва <small>2023 / 134 мин / Южная Корея / 18+</small>
//             </div>
//             <div class="list">
//                 <a href="javascript:ticketManager.richSession(96619587)" class="subitem">
//                     <span class="time">13:20</span>
//                     <span class="badge">2D</span>
//                     <span class="price">170 P</span>
//                 </a>
//             </div>
//         </div>
//     </div>
// </div>
// <div class="step" data-id="1">
//     ...
//     ...
//     ...
// </div>
//
// .step - логический блок
//
//     .place-name - название кинотеатра
//
//     .schedule-item - логический блок
//
//         .title - название фильма
//
//             .small - краткая информация
//
//         .list - логический блок
//
//             .subitem
//
//                 .time - время показа фильма
//
//                 .price - цена билета
//
//              ...
//
//             .subitem
//
//     ...
//
//     .schedule-item
//
// ...

// <div class="description">
// </div>

pub fn parse_text(node: &ElementRef, selector: &Selector) -> Option<String> {
    if let Some(result) = node.select(selector).next() {
        if let Some(text) = result.text().next() {
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }
    None
}

pub fn parse_num_with_regex<T>(text: &str, regex: &Regex) -> Option<T>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    if let Some(captures) = regex.captures(text.trim()) {
        if let Some(matched) = captures.get(1) {
            if let Ok(parsed) = matched.as_str().parse::<T>() {
                return Some(parsed);
            }
        }
    }
    None
}

pub async fn response_get(url: &str) -> CustomResult<Html> {
    // download the target HTML document
    // let response: Result<reqwest::blocking::Response, reqwest::Error> = reqwest::blocking::get(URL_MOSKINO_SCHEDULE);
    // let response = reqwest::get(url).await?;
    // let response = reqwest::blocking::get(url)?;
    let response = reqwest::get(url).await?;

    let html_content = response.text().await?;

    // get the HTML content from the request response
    // let html_content = response.unwrap().text().unwrap();
    // parse the HTML document
    let document = scraper::Html::parse_document(&html_content);
    Ok(document)
}
