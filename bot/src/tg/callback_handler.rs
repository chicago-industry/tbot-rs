use super::*;

pub async fn restart_mainmenu(bot: Bot, dialogue: MyDialogue, msg: Message, date: NaiveDate) -> Res<()> {
    let keyboard = keyboard_main();
    bot.edit_message_text(msg.chat.id, msg.id, "Выберите опцию")
        .reply_markup(keyboard)
        .await?;
    dialogue.update(State::StartOption { date }).await?;
    Ok(())
}

pub async fn restart_dayoption(bot: Bot, dialogue: MyDialogue, msg: Message) -> Res<()> {
    let keyboard = keyboard_day();

    bot.edit_message_text(msg.chat.id, msg.id, "Выберите день")
        .reply_markup(keyboard)
        .await?;
    dialogue.update(State::DayOption).await?;
    Ok(())
}

pub async fn cb_handle_day_option(raw_option: String, msg: Message, bot: Bot, dialogue: MyDialogue) -> Res<()> {
    match raw_option.parse::<i32>() {
        Ok(option) => match option.try_into() {
            Ok(ButtonOption::Close) => {
                bot.delete_message(msg.chat.id, msg.id).await?;
                dialogue.exit().await?;
            }
            _ => {
                error!("E! option: {}, dialogue: {:?}", option, dialogue);
                dialogue.exit().await?;
            }
        },
        Err(_) => match NaiveDate::parse_from_str(&raw_option, "%Y.%m.%d") {
            Ok(date) => {
                let keyboard = keyboard_main();
                bot.edit_message_text(msg.chat.id, msg.id, "Выберите опцию")
                    .reply_markup(keyboard)
                    .await?;
                dialogue.update(State::StartOption { date }).await?;
            }
            Err(_) => {
                error!("E! option: {}, dialogue: {:?}", raw_option, dialogue);
                dialogue.exit().await?;
            }
        },
    }
    Ok(())
}

pub async fn cb_handle_start_option(bot: Bot, dialogue: MyDialogue, msg: Message, db: Arc<DB>, option: i32, date: NaiveDate) -> Res<()> {
    match option.try_into() {
        // option 'Close' selected on the start menu
        Ok(ButtonOption::Close) => {
            bot.delete_message(msg.chat.id, msg.id).await?;
            dialogue.exit().await?;
        }
        Ok(ButtonOption::Up) => {
            restart_dayoption(bot, dialogue, msg).await?;
        }
        // option 'All movies' selected
        Ok(ButtonOption::Movies) => {
            // initialize 'raw' CallbackData for callback_handle_movielist
            let data = CallbackDataDefault::new(date, msg.id, None, *DB_ITEMS_PER_PAGE);
            callback_handle_movielist(bot, dialogue, msg, db, ButtonOption::NotSetted, data).await?;
        }
        // option 'By cinema' selected
        Ok(ButtonOption::Cinemas) => {
            let cinemas = DB::q_get_cinemas(&db.conn).await;

            match cinemas {
                Ok(Some(cinemas)) => {
                    let keyboard = keyboard_cinemas(cinemas);
                    bot.edit_message_text(msg.chat.id, msg.id, "Выберите кинотеатр")
                        .reply_markup(keyboard)
                        .await?;
                    dialogue.update(State::Cinemas { date }).await?;
                }
                Ok(None) => {
                    let keyboard = keyboard_ok_or_up(MenuCode::Cinemas);
                    bot.edit_message_text(msg.chat.id, msg.id, "Нету доступных кинотеатров для показа")
                        .reply_markup(keyboard)
                        .await?;
                    dialogue.update(State::Cinemas { date }).await?;
                }
                Err(e) => {
                    error!("q_get_cinemas: {:?}", e);
                    bot.edit_message_text(msg.chat.id, msg.id, "Что-то пошло не так").await?;
                    dialogue.exit().await?;
                }
            }
        }
        _ => {
            error!("E! option: {}, dialogue: {:?}", option, dialogue);
            dialogue.exit().await?;
        }
    }
    Ok(())
}

pub async fn cb_handle_cinemas(bot: Bot, dialogue: MyDialogue, msg: Message, db: Arc<DB>, option: i32, date: NaiveDate) -> Res<()> {
    match option.try_into() {
        Ok(option) => match option {
            ButtonOption::Close => {
                bot.delete_message(msg.chat.id, msg.id).await?;
                dialogue.exit().await?;
            }
            ButtonOption::Up => {
                restart_mainmenu(bot, dialogue, msg, date).await?;
            }
            _ => {
                error!("What am I doing here");
                dialogue.exit().await?;
            }
        },
        _ => match option {
            cinema_id if cinema_id > 0 => {
                let cinema_name = DB::q_get_cinema_name_by_id(&db.conn, cinema_id).await;

                match cinema_name {
                    Ok(Some(cinema_name)) => {
                        let cinema = Cinema::new(cinema_id, cinema_name);
                        let data = CallbackDataCinema::new(date, cinema, msg.id, None, *DB_ITEMS_PER_PAGE);
                        callback_handle_movielist(bot, dialogue, msg, db, ButtonOption::NotSetted, data).await?;
                    }
                    Ok(None) => {
                        error!("q_get_cinema_name_by_id");

                        bot.edit_message_text(msg.chat.id, msg.id, "Что-то пошло не так").await?;
                        dialogue.exit().await?;
                    }
                    Err(e) => {
                        error!("q_get_cinema_name_by_id: {:?}", e);

                        bot.edit_message_text(msg.chat.id, msg.id, "Что-то пошло не так").await?;
                        dialogue.exit().await?;
                    }
                }
            }
            _ => {
                error!("E! option: {}, dialogue: {:?}", option, dialogue);
                dialogue.exit().await?;
            }
        },
    }
    Ok(())
}

pub async fn cb_handle_pressed_button<T>(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    q: CallbackQuery,
    db: Arc<DB>,
    option: i32,
    mut data: CallbackData<T>,
) -> Res<()>
where
    CallbackData<T>: Cbd,
{
    match option.try_into() {
        Ok(option) => match option {
            ButtonOption::Back | ButtonOption::Forward => {
                bot.answer_callback_query(q.id).await?;
                callback_handle_movielist(bot, dialogue, msg, db, option, data).await?;
            }
            ButtonOption::Close => {
                bot.delete_message(msg.chat.id, msg.id).await?;
                bot.delete_message(msg.chat.id, data.id_msg).await?;
                dialogue.exit().await?;
            }
            ButtonOption::Sessions => {
                let sessions = data.q_get_sessions(&db.conn).await?;
                data.show_sessions(bot, q, sessions).await?;
            }
            ButtonOption::Up => {
                bot.answer_callback_query(q.id).await?;
                data.go_prev(bot, dialogue, msg, db).await?;
            }
            _ => {
                // TODO
            }
        },
        _ => match option {
            // some movie selected on the page
            db_movie_id if db_movie_id > 0 => {
                bot.answer_callback_query(q.id).await?;
                callback_handle_pinned_movie(bot, dialogue, msg, db, db_movie_id, data).await?;
            }
            _ => {
                log::warn!("------------------------------- WHAT AM I DOING HERE {option}");
            }
        },
    }
    Ok(())
}

// processing the list of movies
pub async fn callback_handle_movielist<T>(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    db: Arc<DB>,
    option: ButtonOption,
    mut data: CallbackData<T>,
) -> Res<()>
where
    CallbackData<T>: Cbd,
{
    // if the function was called when clicking the next or previous movie page,
    // then it is necessary to check if we can do this, otherwise do nothing
    if !data.set_current_page(option) {
        return Ok(());
    }

    // TODO:
    // Проверять количество страниц с предыдущим значением
    // если количество страниц изменилось - значит поступили новые данные, или наоборот
    // ЕСЛИ ВЫВЕДЕН КАКОЙ-ТО ФИЛЬМ, ТО ИЛИ УДАЛИТЬ ИЛИ ПРОВЕРИТЬ, ЧТО ОН ЕЩЕ АКТУАЛЕЕН (В ПРОТИВНОМ СЛУЧАЕ УДАЛИТЬ)

    let mut tx = db.conn.begin().await?;

    // find out the current number of available movies
    let db_movies_count = data.q_count_movies(&mut *tx).await?;

    // get brief information about movies
    let movies = data.q_get_movies_short(Arc::clone(&db)).await?;

    tx.commit().await?;

    // check and specify the number of pages
    data.set_total_pages(db_movies_count);

    match movies {
        Some(movies) => {
            // Выводим список фильмов
            let keyboard = keyboard_movielist(movies, data.get_menu_code(), data.db_current_page, data.db_total_pages);

            let text = data.headline_text();

            // TODO:
            // не нужно каждый раз менять текст сообщения, нужно только обновлять клавиатуру (вынести первый вывод сообщения из функции)
            // bot.edit_message_reply_markup(chat.id, data.id_msg).reply_markup(keyboard).await?;
            bot.edit_message_text(msg.chat.id, data.id_msg, text).reply_markup(keyboard).await?;
        }
        None => {
            let (headline_text, keyboard) = data.get_data_for_absence_answer();
            bot.edit_message_text(msg.chat.id, msg.id, headline_text)
                .reply_markup(keyboard)
                .await?;
        }
    }
    dialogue.update(data.state_update()).await?;

    Ok(())
}

// handling of the attached message (movie information) below the list of movies
pub async fn callback_handle_pinned_movie<T>(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    db: Arc<DB>,
    db_movie_id: i32,
    mut data: CallbackData<T>,
) -> Res<()>
where
    CallbackData<T>: Cbd,
{
    // if the current movie is out, then there is no need to update the information
    if let Some(pinned_data) = data.pinned_msg {
        if pinned_data.db_id_movie == db_movie_id {
            return Ok(());
        }
    }

    let movie = DB::q_get_movie_by_id(&db.conn, db_movie_id).await?;

    let text = movie.description();
    let keyboard = keybord_movie_links(movie.href_moskino.as_deref(), movie.href_kinopoisk.as_deref(), data.get_menu_code());

    match data.pinned_msg {
        Some(ref mut pinned_data) => {
            bot.edit_message_text(msg.chat.id, pinned_data.id_msg, text)
                .reply_markup(keyboard)
                .await?;
            pinned_data.db_id_movie = db_movie_id;
        }
        None => {
            let sent = bot.send_message(msg.chat.id, text).reply_markup(keyboard).await?;
            data.pinned_msg = Some(CallbackPinnedMsg::new(sent.id, db_movie_id));
        }
    }

    dialogue.update(data.state_update()).await?;

    Ok(())
}
