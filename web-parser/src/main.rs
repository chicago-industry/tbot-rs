// #![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]
// #![allow(unused_mut)]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;
extern crate db;
extern crate dotenv;
extern crate dotenv_codegen;
extern crate futures;

mod args;
mod moskino;

use clap::Parser;
use lazy_static::lazy_static;
use scraper::Selector;
use std::error::Error;

use db::DB;
use moskino::cinema::MoskinoCinema;
use moskino::movie::MoskinoMovie;
use moskino::session::MoskinoSession;

type CustomError = Box<dyn Error + Send + Sync>;
type CustomResult<T> = Result<T, CustomError>;

static URL_MOSKINO_SCHEDULE: &str = "https://mos-kino.ru/schedule/";

lazy_static! {
    static ref STEP_SELECTOR: Selector = Selector::parse(".step").unwrap();
    // cinemas
    static ref PLACE_SELECTOR: Selector = Selector::parse(".place-name").unwrap();
    // movies
    static ref SCHEDULE_SELECTOR: Selector = Selector::parse(".schedule-item").unwrap();
    // sessions
    static ref SUBITEM_SELECTOR: Selector = Selector::parse(".subitem").unwrap();
}

// TODO
// optimise, make it in real async
#[tokio::main]
async fn main() -> CustomResult<()> {
    let args = args::Args::parse();
    pretty_env_logger::init();

    let db = DB::new().await?;
    info!("Connected to DB");

    let url = args.day.url_by_day();
    let document = moskino::response(&url).await?;

    let mut current_cinema_id;
    let mut current_movie_id;
    // let mut movies: HashSet<MoskinoMovie> = HashSet::new();

    let date = args.day.date();

    info!("------------------------- for [{}] -------------------------", date);

    for node in document.root_element().select(&STEP_SELECTOR) {
        match MoskinoCinema::from_node(node) {
            Ok(cinema) => {
                let cinema = db::Cinema { id: 0, name: cinema.name };
                current_cinema_id = if let Ok(id) = db.insert_cinema(&cinema).await {
                    id
                } else {
                    continue;
                };
            }
            Err(e) => {
                error!("{e}");
                continue;
            }
        }

        for movie_node in node.select(&SCHEDULE_SELECTOR) {
            match MoskinoMovie::from_node(movie_node) {
                Ok(movie) => {
                    let movie = db::Movie {
                        title: movie.title,
                        year: movie.year,
                        genre: movie.genre,
                        director: movie.director,
                        description: movie.description,
                        href_moskino: movie.href_moskino,
                        href_kinopoisk: movie.href_kinopoisk,
                        country: movie.country,
                        duration: movie.duration,
                        age: movie.age,
                        tagline: movie.tagline,
                    };

                    current_movie_id = if let Ok(id) = db.insert_movie(&movie).await {
                        id
                    } else {
                        continue;
                    };
                }
                Err(e) => {
                    error!("{e}");
                    continue;
                }
            }

            for session_node in movie_node.select(&SUBITEM_SELECTOR) {
                match MoskinoSession::from_node(session_node) {
                    Ok(session) => {
                        let session = db::Session {
                            cinema_name: "".into(),
                            showtime: session.time,
                            showdate: date,
                            price: session.price,
                        };

                        let res = db.insert_session(&session, current_cinema_id, current_movie_id).await;

                        match res {
                            Ok(_) => {
                                info!(
                                    "inserted session '{} - {}' for movie: {} and cinema: {}",
                                    session.showtime, session.price, current_movie_id, current_cinema_id
                                );
                            }
                            Err(e) => match e {
                                sqlx::Error::Database(e)
                                    if e.constraint() == Some("sessions_cinema_id_movie_id_showdate_showtime_price_key") =>
                                {
                                    warn!(
                                        "DUPLICATE: '{} - {}' for movie_id: {}, cinema_id: {}",
                                        session.showtime, session.price, current_movie_id, current_cinema_id
                                    );
                                }
                                _ => {
                                    error!("{}", e);
                                }
                            },
                        }
                    }
                    Err(e) => {
                        error!("{e}");
                        continue;
                    }
                }
            }
        }
    }

    info!("------------------------- done -------------------------");
    Ok(())
}
