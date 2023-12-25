use chrono::NaiveTime;
use regex::Regex;
use scraper::{Element, ElementRef, Html, Selector};
use std::{fmt, io};

use super::lazy_static;
use super::CustomResult;

pub mod cinema;
pub mod movie;
pub mod session;

pub(super) fn parse_text(node: &ElementRef, selector: &Selector) -> Option<String> {
    if let Some(result) = node.select(selector).next() {
        if let Some(text) = result.text().next() {
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }
    None
}

pub(super) fn parse_num_with_regex<T>(text: &str, regex: &Regex) -> Option<T>
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

pub(super) async fn response(url: &str) -> CustomResult<Html> {
    let response = reqwest::get(url).await?;
    let html_content = response.text().await?;

    Ok(scraper::Html::parse_document(&html_content))
}

pub(super) fn response_blocking(url: &str) -> CustomResult<Html> {
    let response = reqwest::blocking::get(url)?;
    let html_content = response.text()?;

    Ok(scraper::Html::parse_document(&html_content))
}

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
