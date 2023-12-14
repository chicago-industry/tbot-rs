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
    // id сообщения, в котором выведен список фильмов по кинотеатру
    pub id_msg: MessageId,
    // текущая страница списка
    pub db_current_page: i64,
    // количество страниц
    pub db_total_pages: i64,
    // 'объем' страницы (сколько макс. фильмов на странице)
    pub db_items_per_page: i64,
    // в зависимости из какого меню мы пришли нам будет нужна информация о выбранном кинотеатре:
    // - если внажали кнопку 'Все фильмы', то нам не нужна информация о каком-либо кинотеатре
    // - если нажали кнопку 'По кинотеатру' и впоследствии выбрали кинотеатр, то тут будет хранится информация о выбранном кинотеатре
    pub cinema: T,
    // дополнительное сообщение по фильму (с первого вывода списка фильма отсутствует до тех пор, пока пользователь не нажмет на какой-то фильм)
    pub pinned_msg: Option<CallbackPinnedMsg>,
}

pub type CallbackDataDefault = CallbackData<()>;
pub type CallbackDataCinema = CallbackData<Cinema>;

#[derive(Debug, Copy, Clone)]
pub struct CallbackPinnedMsg {
    // id сообщения, где выведена информация по фильму
    pub id_msg: MessageId,
    // id фильма в сообщении
    pub id_movie: i32,
}

#[async_trait]
// different realization for CallbackDataCinema and CallbackDataDefault
pub trait CallbackDataTrait {
    fn get_menu_code(&self) -> MenuCode;
    fn state_update(self) -> State;
    fn headline_text(&self) -> String;
    fn get_data_for_absence_answer(&self) -> (String, InlineKeyboardMarkup);
    async fn show_sessions(&self, bot: Bot, q: CallbackQuery, sessions: Option<Vec<Session>>) -> CustomResult<()>;
    async fn q_count_movies(&self, conn: impl sqlx::PgExecutor<'_>) -> DBResult<i64>;
    async fn q_get_movies_short(&self, db: Arc<DB>) -> DBResult<Option<Vec<MovieShort>>>;
    async fn q_get_sessions(&mut self, conn: impl sqlx::PgExecutor<'_>) -> DBResult<Option<Vec<Session>>>;
    async fn go_prev(&self, id: MessageId, chat: Chat, bot: Bot, dialogue: MyDialogue, db: Arc<DB>) -> CustomResult<()>;
}

#[async_trait]
impl CallbackDataTrait for CallbackDataDefault {
    async fn go_prev(&self, id: MessageId, chat: Chat, bot: Bot, dialogue: MyDialogue, _: Arc<DB>) -> CustomResult<()> {
        if let Some(pinned_msg) = self.pinned_msg {
            bot.delete_message(chat.id, pinned_msg.id_msg).await?;
        }
        restart_option(id, self.date, chat, bot, dialogue).await
    }

    // TODO
    #[allow(unused_variables)]
    async fn show_sessions(&self, bot: Bot, q: CallbackQuery, sessions: Option<Vec<Session>>) -> CustomResult<()> {
        bot.answer_callback_query(q.id).text(String::from("TODO")).show_alert(true).await?;
        Ok(())
    }

    async fn q_count_movies(&self, conn: impl sqlx::PgExecutor<'_>) -> Result<i64, sqlx::Error> {
        DB::q_count_movies(conn, self.date).await
    }

    // TODO
    // unwrap
    async fn q_get_sessions(&mut self, conn: impl sqlx::PgExecutor<'_>) -> DBResult<Option<Vec<Session>>> {
        DB::q_get_sessions_all(conn, self.pinned_msg.unwrap().id_movie, self.date).await
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
            let s = format!("{} в прокате", self.date.format_localized("%d.%m (%A)", chrono::Locale::ru_RU));
            s.to_lowercase()
        }
    }
}

#[async_trait]
impl CallbackDataTrait for CallbackDataCinema {
    async fn go_prev(&self, id: MessageId, chat: Chat, bot: Bot, dialogue: MyDialogue, db: Arc<DB>) -> CustomResult<()> {
        if let Some(pinned_msg) = self.pinned_msg {
            bot.delete_message(chat.id, pinned_msg.id_msg).await?;
        }
        cb_handle_start_option(ButtonOption::Cinemas as i32, self.date, id, chat, bot, dialogue, db).await
    }

    // TODO
    // check for bytes count (MAX LIMIT)
    async fn show_sessions(&self, bot: Bot, q: CallbackQuery, sessions: Option<Vec<Session>>) -> CustomResult<()> {
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
        DB::q_get_sessions_by_cinema(conn, self.pinned_msg.unwrap().id_movie, self.cinema.id, self.date).await
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
        format!("Фильмы в кинотеатре '{}'", self.cinema.name)
    }
}

impl<T> CallbackData<T> {
    // расчет количества страниц фильмов
    pub fn set_total_pages(&mut self, db_movies_count: i64) {
        let db_current_total_pages = (db_movies_count as f64 / self.db_items_per_page as f64).ceil() as i64;

        // если количество страниц изменилось по сравнению с предыдущим значением (какой-то фильм добавился или пропал),
        // то сбрасываем текущую страницу на 1 и выставляем новый total_pages
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
        !((self.db_current_page == 1 && move_option == ButtonOption::Back) || (self.db_current_page == self.db_total_pages && move_option == ButtonOption::Forward))
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
    pub fn new(id_msg: MessageId, id_movie: i32) -> Self {
        Self { id_msg, id_movie }
    }
}
