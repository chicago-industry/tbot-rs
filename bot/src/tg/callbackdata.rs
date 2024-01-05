use super::*;
use crate::tg::callback_handler::*;
use db::{tools::datetime_utc3, Cinema, DBResult, MovieShort, Session, DB};
use keyboard::*;

// storing data from callbacks (button clicks)
// used for State::FromCinema and State::FromMovie states
#[derive(Debug, Clone)]
pub struct CallbackData<T> {
    // selected date
    pub date: NaiveDate,
    // id a message containing a list of films by cinema
    pub id_msg: MessageId,
    // current movie list page
    pub db_current_page: i64,
    // count of pages
    pub db_total_pages: i64,
    // volume of a page (how many max movies per page)
    pub db_items_per_page: i64,
    // depending on which menu we came from, we will need information about the selected cinema:
    // - if the 'All Movies' button is pressed, we don't need information about any specific cinema.
    // - if the 'By Cinema' button is pressed, and a cinema is subsequently selected, information about the selected cinema will be stored here.
    pub cinema: T,
    // additional movie details (absent from the initial list of movies until the user clicks on a specific movie).
    pub pinned_msg: Option<CallbackPinnedMsg>,
}

pub type CallbackDataDefault = CallbackData<()>;
pub type CallbackDataCinema = CallbackData<Cinema>;

#[derive(Debug, Copy, Clone)]
pub struct CallbackPinnedMsg {
    // ID of the message where movie information is displayed
    pub id_msg: MessageId,
    // ID of the movie in the message
    pub db_id_movie: i32,
}

#[async_trait]
// different realization for CallbackDataCinema and CallbackDataDefault
pub trait Cbd {
    fn get_menu_code(&self) -> MenuCode;
    fn state_update(self) -> State;
    fn headline_text(&self) -> String;
    fn get_data_for_absence_answer(&self) -> (String, InlineKeyboardMarkup);
    async fn show_sessions(&self, bot: Bot, q: CallbackQuery, sessions: Option<Vec<Session>>) -> Res<()>;
    async fn q_count_movies(&self, conn: impl sqlx::PgExecutor<'_>) -> DBResult<i64>;
    async fn q_get_movies_short(&self, db: Arc<DB>) -> DBResult<Option<Vec<MovieShort>>>;
    async fn q_get_sessions(&mut self, conn: impl sqlx::PgExecutor<'_>) -> DBResult<Option<Vec<Session>>>;
    async fn go_prev(&self, bot: Bot, dialogue: MyDialogue, msg: Message, db: Arc<DB>) -> Res<()>;
}

#[async_trait]
impl Cbd for CallbackDataDefault {
    async fn go_prev(&self, bot: Bot, dialogue: MyDialogue, msg: Message, _: Arc<DB>) -> Res<()> {
        if let Some(pinned_msg) = self.pinned_msg {
            bot.delete_message(msg.chat.id, pinned_msg.id_msg).await?;
        }
        restart_mainmenu(bot, dialogue, msg, self.date).await
    }

    // TODO
    // выводить сообщение о переходе на сайт только есть список реально не влезает (сделать + проверить длину)
    #[allow(unused_variables)]
    async fn show_sessions(&self, bot: Bot, q: CallbackQuery, sessions: Option<Vec<Session>>) -> Res<()> {
        bot.answer_callback_query(q.id)
            .text(String::from(
                "Список может получиться слишком длинным. Посмотрите сеансы перейдя по ссылке на Москино",
            ))
            .show_alert(true)
            .await?;
        Ok(())
    }

    async fn q_count_movies(&self, conn: impl sqlx::PgExecutor<'_>) -> Result<i64, sqlx::Error> {
        DB::q_count_movies(conn, self.date).await
    }

    // TODO
    // unwrap
    async fn q_get_sessions(&mut self, conn: impl sqlx::PgExecutor<'_>) -> DBResult<Option<Vec<Session>>> {
        DB::q_get_sessions_all(conn, self.pinned_msg.unwrap().db_id_movie, self.date).await
    }

    async fn q_get_movies_short(&self, db: Arc<DB>) -> Result<Option<Vec<MovieShort>>, sqlx::Error> {
        DB::q_get_movies_short(&db.conn, self.date, self.db_current_page, self.db_items_per_page).await
    }

    fn get_menu_code(&self) -> MenuCode {
        MenuCode::MovielistDefault
    }

    fn get_data_for_absence_answer(&self) -> (String, InlineKeyboardMarkup) {
        let text = "Нету доступных фильмов для показа".to_string();
        let keyboard = keyboard_ok_or_up(MenuCode::MovielistDefault);

        (text, keyboard)
    }

    fn state_update(self) -> State {
        State::FromMovie { data: self }
    }

    fn headline_text(&self) -> String {
        let (curr_date, _) = datetime_utc3();

        if curr_date == self.date {
            "Сегодня в прокате".to_string()
        } else {
            format!("{} в прокате", self.date.format_localized("%d.%m (%A)", chrono::Locale::ru_RU)).to_lowercase()
        }
    }
}

#[async_trait]
impl Cbd for CallbackDataCinema {
    async fn go_prev(&self, bot: Bot, dialogue: MyDialogue, msg: Message, db: Arc<DB>) -> Res<()> {
        if let Some(pinned_msg) = self.pinned_msg {
            bot.delete_message(msg.chat.id, pinned_msg.id_msg).await?;
        }
        cb_handle_start_option(bot, dialogue, msg, db, ButtonOption::Cinemas as i32, self.date).await
    }

    // TODO
    // check for bytes count (MAX LIMIT)
    async fn show_sessions(&self, bot: Bot, q: CallbackQuery, sessions: Option<Vec<Session>>) -> Res<()> {
        let text: String = match sessions {
            Some(s) => s
                .iter()
                .map(|s| format!("{} - {} руб.", s.showtime.format("%H:%M"), s.price))
                .collect::<Vec<String>>()
                .join(" | "),
            None => String::from("Нету доступных сеансов"),
        };

        bot.answer_callback_query(q.id).text(text).show_alert(true).await?;
        Ok(())
    }

    // TODO
    // unwrap
    async fn q_get_sessions(&mut self, conn: impl sqlx::PgExecutor<'_>) -> DBResult<Option<Vec<Session>>> {
        DB::q_get_sessions_by_cinema(conn, self.pinned_msg.unwrap().db_id_movie, self.cinema.id, self.date).await
    }

    async fn q_count_movies(&self, conn: impl sqlx::PgExecutor<'_>) -> Result<i64, sqlx::Error> {
        DB::q_count_movies_by_cinema(conn, self.date, self.cinema.id).await
    }

    async fn q_get_movies_short(&self, db: Arc<DB>) -> Result<Option<Vec<MovieShort>>, sqlx::Error> {
        DB::q_get_movies_short_by_cinema(&db.conn, self.date, self.cinema.id, self.db_current_page, self.db_items_per_page).await
    }

    fn get_menu_code(&self) -> MenuCode {
        MenuCode::MovielistFromCinema
    }

    fn get_data_for_absence_answer(&self) -> (String, InlineKeyboardMarkup) {
        let text = format!("В кинотеатре '{}' нету доступных фильмов для показа", self.cinema.name);
        let keyboard = keyboard_ok_or_up(MenuCode::MovielistFromCinema);

        (text, keyboard)
    }

    fn state_update(self) -> State {
        State::FromCinema { data: self }
    }

    fn headline_text(&self) -> String {
        let (curr_date, _) = datetime_utc3();

        if curr_date == self.date {
            format!("Сегодня в кинотеатре {}", self.cinema.name)
        } else {
            let text_date = format!("{}", self.date.format_localized("%d.%m (%A)", chrono::Locale::ru_RU),).to_lowercase();
            format!("{} в кинотеатре {}", text_date, self.cinema.name)
        }
    }
}

impl<T> CallbackData<T> {
    // calculation of the number of movie pages
    pub fn set_total_pages(&mut self, db_movies_count: i64) {
        let db_current_total_pages = (db_movies_count as f64 / self.db_items_per_page as f64).ceil() as i64;

        // if the number of pages has changed compared to the previous value (a movie has been added or removed),
        // then reset the current page to 1 and set the new total_pages
        if self.db_total_pages != db_current_total_pages {
            self.db_current_page = 1;
            self.db_total_pages = db_current_total_pages;
        }
    }

    pub fn set_current_page(&mut self, option: ButtonOption) -> bool {
        if self.can_step(option) {
            self.db_current_page += match option {
                ButtonOption::Back => -1,
                ButtonOption::Forward => 1,
                _ => 0,
            };
            return true;
        }
        false
    }

    fn can_step(&self, move_option: ButtonOption) -> bool {
        !((self.db_current_page == 1 && move_option == ButtonOption::Back)
            || (self.db_current_page == self.db_total_pages && move_option == ButtonOption::Forward))
    }
}

impl CallbackDataCinema {
    pub fn new(date: NaiveDate, cinema: Cinema, id_msg: MessageId, pinned_msg: Option<CallbackPinnedMsg>, db_items_per_page: i64) -> Self {
        Self {
            date,
            id_msg,
            pinned_msg,
            db_current_page: 1,
            db_items_per_page,
            db_total_pages: 0,
            cinema,
        }
    }
}
impl CallbackDataDefault {
    pub fn new(date: NaiveDate, id_msg: MessageId, pinned_msg: Option<CallbackPinnedMsg>, db_items_per_page: i64) -> Self {
        Self {
            date,
            id_msg,
            pinned_msg,
            db_current_page: 1,
            db_items_per_page,
            db_total_pages: 0,
            cinema: (),
        }
    }
}

impl CallbackPinnedMsg {
    pub fn new(id_msg: MessageId, db_id_movie: i32) -> Self {
        Self { id_msg, db_id_movie }
    }
}
