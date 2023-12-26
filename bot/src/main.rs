// #![allow(unused_imports)]
// #![allow(dead_code)]
// #![allow(unused_variables)]
// #![allow(unused_mut)]

#[macro_use]
extern crate dotenv_codegen;
extern crate chrono;
extern crate db;
extern crate dotenv;
extern crate lazy_static;
extern crate log;
extern crate teloxide;
extern crate tokio;

// mod db;
mod tg;

// use crate::db;
// use db;

use chrono::NaiveDate;
use lazy_static::lazy_static;
use log::{error, info};
use std::{convert::TryFrom, error::Error, io, sync::Arc};
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage},
    payloads::SendMessageSetters,
    prelude::*,
    types::{Chat, InlineKeyboardButton, InlineKeyboardMarkup, Me, MessageId},
    utils::command::BotCommands,
};

use db::{Cinema, Movie, MovieShort, DB};
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

// TODO:
// start to use tokio async await dude
#[tokio::main]
async fn main() -> Res<()> {
    pretty_env_logger::init();

    let db = DB::pool().await?;
    let db = Arc::new(db);
    info!("DB: connected");

    let bot = Bot::new(dotenv!("TELOXIDE_TOKEN"));
    info!("TG: token accepted");

    let handler = dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new(), db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

pub async fn message_handler(bot: Bot, dialogue: MyDialogue, msg: Message, me: Me) -> Res<()> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                let keyboard = keyboard_day();

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

async fn callback_handler(bot: Bot, dialogue: MyDialogue, q: CallbackQuery, db: Arc<DB>) -> Res<()> {
    // TODO
    // pass CallbackQuery to functions instead of id, chat
    if let (Some(callback_data), Some(Message { id, chat, .. })) = (q.data.clone(), q.message.clone()) {
        let (menu_code, raw_option) = parse_callback_data(&callback_data)?;
        let state: Option<State> = dialogue.get().await?;

        match state {
            Some(State::DayOption) => {
                bot.answer_callback_query(q.id).await?;
                menu_code.check_complience(MenuCode::ChooseDay)?;

                cb_handle_day_option(raw_option, id, chat, bot, dialogue).await?;
            }
            Some(State::StartOption { date }) => {
                bot.answer_callback_query(q.id).await?;
                menu_code.check_complience(MenuCode::MainMenu)?;

                let option = raw_option.parse::<i32>()?;
                cb_handle_start_option(option, date, id, chat, bot, dialogue, db).await?;
            }
            Some(State::Cinemas { date }) => {
                bot.answer_callback_query(q.id).await?;
                menu_code.check_complience(MenuCode::Cinemas)?;

                let option = raw_option.parse::<i32>()?;
                cb_handle_cinemas(option, date, id, chat, bot, dialogue, db).await?;
            }
            Some(State::FromCinema { data }) => {
                menu_code.check_complience(MenuCode::MovielistFromCinema)?;

                let option = raw_option.parse::<i32>()?;
                cb_handle_pressed_button(data, option, id, chat, bot, dialogue, db, q).await?;
            }
            Some(State::FromMovie { data }) => {
                menu_code.check_complience(MenuCode::MovielistDefault)?;

                let option = raw_option.parse::<i32>()?;
                cb_handle_pressed_button(data, option, id, chat, bot, dialogue, db, q).await?;
            }
            _ => {
                error!("state is none");
                dialogue.exit().await?;
            }
        }
    }
    Ok(())
}
