#[macro_use]
extern crate log;
extern crate db;

use chrono::NaiveDate;
use clap::Parser;
use db::DB;
use dotenv_codegen::dotenv;
use lazy_static::lazy_static;
use log::{error, info};
use scraper::{Html, Selector};
use std::error::Error;
use std::sync::Arc;

mod args;
mod moskino;

use moskino::cinema::MoskinoCinema;
use moskino::movie::MoskinoMovie;
use moskino::session::MoskinoSession;

type Errr = Box<dyn Error + Send + Sync>;
type Res<T> = Result<T, Errr>;

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

#[tokio::main]
async fn main() -> Res<()> {
    let args = args::Args::parse();
    let date = args.day.date();

    pretty_env_logger::init();

    let db = DB::new(dotenv!("DATABASE_URL")).await?;
    let db = Arc::new(db);
    info!("DB: connected");

    sqlx::migrate!("../db/migrations").run(&db.conn).await?;

    info!("Trying to parse by date {}", date);

    let url = args.day.url_by_day();
    let document = moskino::response(&url).await?;

    // collect cinema html blocks
    let content = document
        .root_element()
        .select(&STEP_SELECTOR)
        .map(|node| node.inner_html())
        .collect::<Vec<String>>();

    scrap_cinemas(db, date, content).await;

    info!("done");
    Ok(())
}

async fn scrap_cinemas(db: Arc<DB>, date: NaiveDate, content: Vec<String>) {
    let mut handlers = vec![];

    for cinema_node in content {
        let db = Arc::clone(&db);

        handlers.push(tokio::spawn(async move {
            let db2 = Arc::clone(&db);

            // TODO handle result
            let cinema_id = parse_cinema(db, &cinema_node).await.unwrap();

            // collect movie html blocks for every cinema
            let content = Html::parse_document(&cinema_node)
                .root_element()
                .select(&SCHEDULE_SELECTOR)
                .map(|node| node.inner_html())
                .collect::<Vec<String>>();

            scrap_movies(db2, cinema_id, date, content).await;
        }));
    }

    for task in handlers {
        task.await.unwrap();
    }
}

async fn scrap_movies(db: Arc<DB>, cinema_id: i32, date: NaiveDate, content: Vec<String>) {
    let mut handlers = vec![];

    for movie_node in content {
        let db = Arc::clone(&db);

        handlers.push(tokio::task::spawn(async move {
            let db2 = Arc::clone(&db);

            // TODO handle result
            let movie_id = parse_movie(db, &movie_node).await.unwrap();

            // collect session html blocks for every movie
            let content = Html::parse_document(&movie_node)
                .root_element()
                .select(&SUBITEM_SELECTOR)
                .map(|node| node.inner_html())
                .collect::<Vec<String>>();

            scrap_session(db2, cinema_id, movie_id, date, content).await;
        }));
    }

    for task in handlers {
        task.await.unwrap();
    }
}

async fn scrap_session(db: Arc<DB>, cinema_id: i32, movie_id: i32, date: NaiveDate, content: Vec<String>) {
    let mut handlers = vec![];

    for session in content {
        let db = Arc::clone(&db);

        handlers.push(tokio::task::spawn(async move {
            parse_session(db, cinema_id, movie_id, date, &session).await;
        }));
    }

    for task in handlers {
        task.await.unwrap();
    }
}

async fn parse_session(db: Arc<DB>, cinema_id: i32, movie_id: i32, date: NaiveDate, session_node: &str) {
    match MoskinoSession::from_node(session_node) {
        Ok(session) => {
            // temp wrap
            let session = db::Session {
                cinema_name: "".into(),
                showtime: session.time,
                showdate: date,
                price: session.price,
            };

            let res = db.insert_session(&session, cinema_id, movie_id).await;

            match res {
                Ok(_) => {
                    info!(
                        "inserted session '{} - {}' for movie: {} and cinema: {}",
                        session.showtime, session.price, movie_id, cinema_id
                    );
                }
                Err(e) => match e {
                    sqlx::Error::Database(e) if e.constraint() == Some("sessions_cinema_id_movie_id_showdate_showtime_price_key") => {
                        warn!(
                            "DUPLICATE: '{} - {}' for movie_id: {}, cinema_id: {}",
                            session.showtime, session.price, movie_id, cinema_id
                        );
                    }
                    _ => {
                        error!("{}", e);
                    }
                },
            }
        }
        Err(e) => {
            error!("{}", e);
        }
    }
}

async fn parse_cinema(db: Arc<DB>, cinema_node: &str) -> Res<i32> {
    match MoskinoCinema::from_node(cinema_node) {
        Ok(cinema) => {
            info!("{}", cinema.name);

            // temp wrap
            let cinema = db::Cinema { id: 0, name: cinema.name };

            Ok(db.insert_cinema(&cinema).await?)
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

async fn parse_movie(db: Arc<DB>, movie_node: &str) -> Res<i32> {
    match MoskinoMovie::from_node(movie_node) {
        Ok(movie) => {
            info!("{}", movie.title);

            // temp wrap
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

            Ok(db.insert_movie(&movie).await?)
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}
