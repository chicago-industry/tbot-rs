use super::*;

pub async fn restart_option(id: MessageId, date: NaiveDate, chat: Chat, bot: Bot, dialogue: MyDialogue) -> CustomResult<()> {
    let keyboard = keyboard_main();
    bot.edit_message_text(chat.id, id, "Выберите опцию").reply_markup(keyboard).await?;
    dialogue.update(State::StartOption { date }).await?;
    Ok(())
}

pub async fn restart_day(id: MessageId, chat: Chat, bot: Bot, dialogue: MyDialogue) -> CustomResult<()> {
    let keyboard = keyboard_day();
    bot.edit_message_text(chat.id, id, "Выберите день").reply_markup(keyboard).await?;
    dialogue.update(State::DayOption).await?;
    Ok(())
}

pub async fn cb_handle_day_option(raw_option: String, id: MessageId, chat: Chat, bot: Bot, dialogue: MyDialogue) -> CustomResult<()> {
    match raw_option.parse::<i32>() {
        Ok(option) => match option.try_into() {
            Ok(ButtonOption::Close) => {
                bot.delete_message(chat.id, id).await?;
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
                bot.edit_message_text(chat.id, id, "Выберите опцию").reply_markup(keyboard).await?;
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

pub async fn cb_handle_start_option(option: i32, date: NaiveDate, id: MessageId, chat: Chat, bot: Bot, dialogue: MyDialogue, db: Arc<DB>) -> CustomResult<()> {
    match option.try_into() {
        // Выбрана опция 'Закрыть' на стартовом меню
        Ok(ButtonOption::Close) => {
            bot.delete_message(chat.id, id).await?;
            dialogue.exit().await?;
        }
        Ok(ButtonOption::Up) => {
            restart_day(id, chat, bot, dialogue).await?;
        }
        // Выбрана опция 'Все фильмы'
        Ok(ButtonOption::Movies) => {
            // Иницилизируем 'сырую' CallbackData для callback_handle_movielist
            let data = CallbackDataDefault::new(date, id, None, *DB_ITEMS_PER_PAGE);
            callback_handle_movielist(data, ButtonOption::NotSetted, id, chat, bot, dialogue, db).await?;
        }
        // Выбрана опция 'По кинотеатру'
        Ok(ButtonOption::Cinemas) => {
            let cinemas = DB::q_get_cinemas(&db.conn).await;

            match cinemas {
                Ok(Some(cinemas)) => {
                    let keyboard = keyboard_cinemas(cinemas);
                    bot.edit_message_text(chat.id, id, "Выберите кинотеатр").reply_markup(keyboard).await?;
                    dialogue.update(State::Cinemas { date }).await?;
                }
                Ok(None) => {
                    let keyboard = keyboard_ok_or_up(MenuCode::Cinemas);
                    bot.edit_message_text(chat.id, id, "Нету доступных кинотеатров для показа").reply_markup(keyboard).await?;
                    dialogue.update(State::Cinemas { date }).await?;
                }
                Err(e) => {
                    error!("q_get_cinemas: {:?}", e);
                    bot.edit_message_text(chat.id, id, "Что-то пошло не так").await?;
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

pub async fn cb_handle_cinemas(option: i32, date: NaiveDate, id: MessageId, chat: Chat, bot: Bot, dialogue: MyDialogue, db: Arc<DB>) -> CustomResult<()> {
    match option.try_into() {
        Ok(option) => match option {
            ButtonOption::Close => {
                bot.delete_message(chat.id, id).await?;
                dialogue.exit().await?;
            }
            ButtonOption::Up => {
                restart_option(id, date, chat, bot, dialogue).await?;
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
                        let data = CallbackDataCinema::new(date, cinema, id, None, *DB_ITEMS_PER_PAGE);
                        callback_handle_movielist(data, ButtonOption::NotSetted, id, chat, bot, dialogue, db).await?;
                    }
                    Ok(None) => {
                        error!("q_get_cinema_name_by_id");
                        bot.edit_message_text(chat.id, id, "Что-то пошло не так").await?;
                        dialogue.exit().await?;
                    }
                    Err(e) => {
                        error!("q_get_cinema_name_by_id: {:?}", e);
                        bot.edit_message_text(chat.id, id, "Что-то пошло не так").await?;
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

#[allow(clippy::too_many_arguments)]
pub async fn cb_handle_pressed_button<T>(
    mut data: CallbackData<T>,
    option: i32,
    id: MessageId,
    chat: Chat,
    bot: Bot,
    dialogue: MyDialogue,
    db: Arc<DB>,
    q: CallbackQuery,
) -> CustomResult<()>
where
    CallbackData<T>: CallbackDataTrait,
{
    match option.try_into() {
        Ok(option) => match option {
            ButtonOption::Back | ButtonOption::Forward => {
                bot.answer_callback_query(q.id).await?;
                callback_handle_movielist(data, option, id, chat, bot, dialogue, db).await?;
            }
            ButtonOption::Close => {
                bot.delete_message(chat.id, id).await?;
                bot.delete_message(chat.id, data.id_msg).await?;
                dialogue.exit().await?;
            }
            ButtonOption::Sessions => {
                let sessions = data.q_get_sessions(&db.conn).await?;
                data.show_sessions(bot, q, sessions).await?;
            }
            ButtonOption::Up => {
                bot.answer_callback_query(q.id).await?;
                data.go_prev(id, chat, bot, dialogue, db).await?;
            }
            _ => {
                //
            }
        },
        _ => match option {
            // Выбран фильм на странице
            x if x > 0 => {
                bot.answer_callback_query(q.id).await?;
                callback_handle_pinned_movie(data, x, chat, bot, dialogue, db).await?;
            }
            _ => {
                log::warn!("------------------------------- WHAT AM I DOING HERE {option}");
            }
        },
    }
    Ok(())
}

// обработка списка фильмов
pub async fn callback_handle_movielist<T>(
    mut data: CallbackData<T>,
    option: ButtonOption,
    id: MessageId,
    chat: Chat,
    bot: Bot,
    dialogue: MyDialogue,
    db: Arc<DB>,
) -> CustomResult<()>
where
    CallbackData<T>: CallbackDataTrait,
{
    // если функция была вызвана при нажатии следующей или предыдущей страницы фильмов,
    // то необходимо проверить, что мы можем это сделать, в противном случае ничего не делаем
    if !data.set_current_page(option) {
        return Ok(());
    }

    // TODO:
    // Проверять количество страниц с предыдущим значением
    // если количество страниц изменилось - значит поступили новые данные, или наоборот
    // ЕСЛИ ВЫВЕДЕН КАКОЙ-ТО ФИЛЬМ, ТО ИЛИ УДАЛИТЬ ИЛИ ПРОВЕРИТЬ, ЧТО ОН ЕЩЕ АКТУАЛЕЕН (В ПРОТИВНОМ СЛУЧАЕ УДАЛИТЬ)

    let mut tx = db.conn.begin().await?;

    // узнаем текущее количество доступных фильмов
    let db_movies_count = data.q_count_movies(&mut *tx).await?;

    // получаем краткую информацию по фильмам
    let movies = data.q_get_movies_short(Arc::clone(&db)).await?;

    tx.commit().await?;

    // проверяем и указываем количество страниц
    data.set_total_pages(db_movies_count);

    match movies {
        Some(movies) => {
            // Выводим список фильмов
            let keyboard = keyboard_movielist(movies, data.get_menu_code(), data.db_current_page, data.db_total_pages);

            let text = data.headline_text();

            // TODO:
            // не нужно каждый раз менять текст сообщения, нужно только обновлять клавиатуру (вынести первый вывод сообщения из функции)
            // bot.edit_message_reply_markup(chat.id, data.id_msg).reply_markup(keyboard).await?;
            bot.edit_message_text(chat.id, data.id_msg, text).reply_markup(keyboard).await?;
        }
        None => {
            let (headline_text, keyboard) = data.get_data_for_absence_answer();
            bot.edit_message_text(chat.id, id, headline_text).reply_markup(keyboard).await?;
        }
    }
    // Обновляем стейт
    dialogue.update(data.state_update()).await?;
    Ok(())
}

// обработка прикреплённого сообщения (информация по фильму) под списком фильмов
// TODO
// movie.unwrap()
pub async fn callback_handle_pinned_movie<T>(mut data: CallbackData<T>, movie_id: i32, chat: Chat, bot: Bot, dialogue: MyDialogue, db: Arc<DB>) -> CustomResult<()>
where
    CallbackData<T>: CallbackDataTrait,
{
    // if the current movie is out, then there is no need to update the information
    if let Some(pinned_data) = data.pinned_msg {
        if pinned_data.id_movie == movie_id {
            return Ok(());
        }
    }

    let movie = DB::q_get_movie_by_id(&db.conn, movie_id).await?;

    let text = create_movie_description(&movie);
    let keyboard = keybord_movie_links(movie.href_moskino.as_deref(), movie.href_kinopoisk.as_deref(), data.get_menu_code());

    match data.pinned_msg {
        Some(ref mut pinned_data) => {
            bot.edit_message_text(chat.id, pinned_data.id_msg, text).reply_markup(keyboard).await?;
            pinned_data.id_movie = movie_id;
        }
        None => {
            let sent = bot.send_message(chat.id, text).reply_markup(keyboard).await?;
            data.pinned_msg = Some(CallbackPinnedMsg::new(sent.id, movie_id));
        }
    }

    dialogue.update(data.state_update()).await?;

    Ok(())
}
