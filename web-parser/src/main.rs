#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;
extern crate dotenv;
#[macro_use]
extern crate dotenv_codegen;
extern crate db;

use chrono::{Duration, NaiveDate, Utc};
use clap::builder::Str;
use clap::{Parser, ValueEnum};
use scraper::Html;
use std::error::Error;

mod moskino;

use db::{Cinema, Movie, MovieShort, DB};
use moskino::cinema::MoskinoCinema;
use moskino::*;

static URL_MOSKINO_SCHEDULE: &str = "https://mos-kino.ru/schedule/";

type CustomError = Box<dyn Error + Send + Sync>;
type CustomResult<T> = Result<T, CustomError>;

#[derive(Debug, Parser)]
struct ProgramArgs {
    /// What day to parse movies
    #[arg(short, long, default_value = "today")]
    #[clap(value_enum)]
    day: ArgDay,
}

#[derive(Debug, ValueEnum, Clone)]
enum ArgDay {
    Today,
    Tommorow,
    Aftertommorow,
}

impl ArgDay {
    fn get_date(&self) -> NaiveDate {
        match self {
            Self::Today => {
                //
                (Utc::now() + Duration::hours(3)).date_naive()
            }
            Self::Tommorow => {
                //
                (Utc::now() + Duration::hours(3) + Duration::days(1)).date_naive()
            }
            Self::Aftertommorow => {
                //
                (Utc::now() + Duration::hours(3) + Duration::days(2)).date_naive()
            }
        }
    }

    fn get_url_by_day(&self) -> String {
        format!("{}?date={}", URL_MOSKINO_SCHEDULE, self.get_date().format("%Y-%m-%d"))
    }
}

// TODO:
// прочитать уже наконец про tokio и сделать норм асинхронный код с распараллеливанием, пулами, блэкджеком и остальным
// передeлать на HashMap по фильмам, кинотетрам и сеансам
// refactoring, optimising
#[tokio::main]
async fn main() -> CustomResult<()> {
    let args = ProgramArgs::parse();
    pretty_env_logger::init();

    info!("------------------------- for [{}] -------------------------", args.day.get_date());

    let document: Html = match response_get(&args.day.get_url_by_day()).await {
        Ok(parsed_html) => parsed_html,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };

    let db = DB::new().await?;
    info!("Connected to DB");

    // Парсим Москино
    let cinemas = MoskinoCinema::parse_vec(document.root_element()).await;
    let date = args.day.get_date();

    for c in cinemas {
        let cinema = db::Cinema { id: 0, name: c.name };

        let cinema_id = if let Ok(id) = db.insert_cinema(&cinema).await {
            id
        } else {
            continue;
        };

        for m in c.movies {
            let movie = db::Movie {
                title: m.title,
                year: m.year,
                genre: m.genre,
                director: m.director,
                description: m.description,
                href_moskino: m.link_moskino,
                href_kinopoisk: m.link_kinopoisk,
                country: m.country,
                duration: m.duration,
                age: m.age,
                tagline: m.tagline,
            };

            let movie_id = if let Ok(id) = db.insert_movie(&movie).await {
                id
            } else {
                continue;
            };

            for s in m.showings {
                let session = db::Session {
                    cinema_name: "".to_string(),
                    showtime: s.time,
                    showdate: date,
                    price: s.price,
                };

                let res = db.insert_session(&session, cinema_id, movie_id).await;

                match res {
                    Ok(r) => {
                        info!("Inserted session {:?} for movie: {} and cinema: {}", session, movie_id, cinema_id);
                    }
                    Err(e) => match e {
                        // TODO
                        // warn for duplicate
                        _ => {
                            error!("{}", e);
                        }
                    },
                }
            }
        }
    }

    // info!("------------------------- done -------------------------");

    Ok(())
}
