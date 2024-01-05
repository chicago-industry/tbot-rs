extern crate db;

use chrono::NaiveDate;
use db::{Cinema, MovieShort, DB};
use dotenv_codegen::dotenv;
use lazy_static::lazy_static;
use log::{error, info};
use std::{convert::TryFrom, error::Error, io, sync::Arc};
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage},
    payloads::SendMessageSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me, MessageId},
    utils::command::BotCommands,
};

mod tg;

use tg::callback_handler::*;
use tg::callbackdata::*;
use tg::keyboard::*;
use tg::tools::*;

lazy_static! {
    static ref DB_ITEMS_PER_PAGE: i64 = dotenv!("DB_ITEMS_PER_PAGE").parse().unwrap();
}

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
pub type Errr = Box<dyn Error + Send + Sync>;
pub type Res<T> = Result<T, Errr>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    Start,
}

#[derive(Debug, Default, Clone)]
pub enum State {
    #[default]
    DayOption,
    StartOption {
        date: NaiveDate,
    },
    Cinemas {
        date: NaiveDate,
    },
    FromCinema {
        data: CallbackDataCinema,
    },
    FromMovie {
        data: CallbackDataDefault,
    },
}

#[tokio::main]
async fn main() -> Res<()> {
    println!("DATABASE_URL: {:?}", dotenv!("DATABASE_URL"));
    println!("DATABASE_URL: {:?}", dotenv!("TELOXIDE_TOKEN"));
    println!("DATABASE_URL: {:?}", dotenv!("DATABASE_MAX_CONNECTIONS"));
    println!("DATABASE_URL: {:?}", dotenv!("DB_ITEMS_PER_PAGE"));

    pretty_env_logger::init();

    let db = DB::pool(dotenv!("DATABASE_URL"), dotenv!("DATABASE_MAX_CONNECTIONS").parse().unwrap()).await?;
    let db = Arc::new(db);
    info!("DB: connected");

    sqlx::migrate!("../db/migrations").run(&db.conn).await?;

    let bot = Bot::new(dotenv!("TELOXIDE_TOKEN"));
    info!("TG: token accepted");

    let handler = dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(
            Update::filter_callback_query()
                // there is should be sub branch in case to split handlers by statements
                .endpoint(callback_handler),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new(), db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

pub async fn message_handler(bot: Bot, dialogue: MyDialogue, msg: Message, me: Me, db: Arc<DB>) -> Res<()> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                let keyboard = keyboard_day();

                // TODO
                let _ = db.insert_user(msg.chat.id.0, msg.chat.username()).await;

                bot.send_message(msg.chat.id, "Выберите день").reply_markup(keyboard).await?;
                dialogue.update(State::DayOption).await?;
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Команда не найдена!").await?;
                dialogue.exit().await?;
            }
        }
    }
    Ok(())
}

fn callback_get(q: CallbackQuery) -> Res<(MenuCode, String, Message)> {
    match (q.data, q.message) {
        (Some(data), Some(message)) => {
            let (menu_code, menu_option) = callback_parse(&data)?;
            Ok((menu_code, menu_option, message))
        }
        _ => Err(Box::new(io::Error::new(io::ErrorKind::InvalidInput, "No callback data or message"))),
    }
}

async fn callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery, db: Arc<DB>) -> Res<()> {
    let (m_code, m_opt, message) = callback_get(q.clone())?;
    let state: Option<State> = dialogue.get().await?;

    match state {
        Some(State::DayOption) => {
            bot.answer_callback_query(q.id).await?;
            m_code.check_complience(MenuCode::ChooseDay)?;

            cb_handle_day_option(m_opt, message, bot, dialogue).await?;
        }
        Some(State::StartOption { date }) => {
            bot.answer_callback_query(q.id).await?;
            m_code.check_complience(MenuCode::MainMenu)?;

            let option = m_opt.parse::<i32>()?;
            cb_handle_start_option(bot, dialogue, message, db, option, date).await?;
        }
        Some(State::Cinemas { date }) => {
            bot.answer_callback_query(q.id).await?;
            m_code.check_complience(MenuCode::Cinemas)?;

            let option = m_opt.parse::<i32>()?;
            cb_handle_cinemas(bot, dialogue, message, db, option, date).await?;
        }
        Some(State::FromCinema { data }) => {
            m_code.check_complience(MenuCode::MovielistFromCinema)?;

            let option = m_opt.parse::<i32>()?;
            cb_handle_pressed_button(bot, dialogue, message, q, db, option, data).await?;
        }
        Some(State::FromMovie { data }) => {
            m_code.check_complience(MenuCode::MovielistDefault)?;

            let option = m_opt.parse::<i32>()?;
            cb_handle_pressed_button(bot, dialogue, message, q, db, option, data).await?;
        }
        _ => {
            error!("Couldn't get State");
            dialogue.exit().await?;
        }
    }

    Ok(())
}
