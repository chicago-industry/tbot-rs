use super::*;
use db::ArgDay;
use url::Url;

pub const CD_DELIMETER: char = ':';

// menu identifier
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum MenuCode {
    #[default]
    ChooseDay,
    MainMenu,
    Cinemas,
    MovielistFromCinema,
    MovielistDefault,
    PinnedMovie,
}

impl MenuCode {
    pub fn check_complience(self, code: MenuCode) -> Res<()> {
        if self != code {
            Err(Box::new(io::Error::new(io::ErrorKind::Other, "Invalid MenuCode number")))
        } else {
            Ok(())
        }
    }
}

impl TryFrom<i32> for MenuCode {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        use MenuCode::*;
        match v {
            x if x == MainMenu as i32 => Ok(MainMenu),
            x if x == Cinemas as i32 => Ok(Cinemas),
            x if x == MovielistFromCinema as i32 => Ok(MovielistFromCinema),
            x if x == MovielistDefault as i32 => Ok(MovielistDefault),
            x if x == ChooseDay as i32 => Ok(ChooseDay),
            x if x == PinnedMovie as i32 => Ok(PinnedMovie),
            _ => Err(()),
        }
    }
}

// Идентификатор для нажатой управляющей кнопки (Назад, Вперед, Закрыть и т.д)
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ButtonOption {
    // Выбрать день
    Day = -100,
    // Все фильмы
    Movies,
    // По кинотеатру
    Cinemas,
    // Назад
    Back,
    // Вперед
    Forward,
    // Назад (вверх, на предыдущее меню)
    Up,
    // Закрыть
    Close,
    //
    Sessions,
    // TODO
    // Страницы (не исп.)
    Pages,
    // TODO
    // (не исп.)
    NotSetted,
}

impl TryFrom<i32> for ButtonOption {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        use ButtonOption::*;
        match v {
            x if x == Movies as i32 => Ok(Movies),
            x if x == Cinemas as i32 => Ok(Cinemas),
            x if x == Back as i32 => Ok(Back),
            x if x == Forward as i32 => Ok(Forward),
            x if x == Up as i32 => Ok(Up),
            x if x == Close as i32 => Ok(Close),
            x if x == Pages as i32 => Ok(Pages),
            x if x == Sessions as i32 => Ok(Sessions),
            _ => Err(()),
        }
    }
}

pub fn keyboard_day() -> InlineKeyboardMarkup {
    let today = ArgDay::get_date(ArgDay::Today);
    let tommorow = ArgDay::get_date(ArgDay::Tommorow);
    let aftertommorow = ArgDay::get_date(ArgDay::Aftertommorow);

    let callback_data = format!("{}{}{}", MenuCode::ChooseDay as i32, CD_DELIMETER, today.format("%Y.%m.%d"));
    let button_1 = InlineKeyboardButton::callback("Сегодня", callback_data);

    let text = format!("{}", tommorow.format_localized("%A • %d.%m", chrono::Locale::ru_RU));
    let callback_data = format!("{}{}{}", MenuCode::ChooseDay as i32, CD_DELIMETER, tommorow.format("%Y.%m.%d"));
    let button_2 = InlineKeyboardButton::callback(text, callback_data);

    let text = format!("{}", aftertommorow.format_localized("%A • %d.%m", chrono::Locale::ru_RU));
    let callback_data = format!("{}{}{}", MenuCode::ChooseDay as i32, CD_DELIMETER, aftertommorow.format("%Y.%m.%d"));
    let button_3 = InlineKeyboardButton::callback(text, callback_data);

    let callback_data = format!("{}{}{}", MenuCode::ChooseDay as i32, CD_DELIMETER, ButtonOption::Close as i32);
    let button_4 = InlineKeyboardButton::callback("❌ Закрыть", callback_data);

    InlineKeyboardMarkup::new(vec![vec![button_1], vec![button_2, button_3], vec![button_4]])
}

// Стартовое меню кнопок для выбора опции
// | Все фильмы | По кинотеатру |
// |       ❌ Закрыть           |
pub fn keyboard_main() -> InlineKeyboardMarkup {
    let callback_data = format!("{}{}{}", MenuCode::MainMenu as i32, CD_DELIMETER, ButtonOption::Movies as i32);
    let button_1 = InlineKeyboardButton::callback("Все фильмы", callback_data);

    let callback_data = format!("{}{}{}", MenuCode::MainMenu as i32, CD_DELIMETER, ButtonOption::Cinemas as i32);
    let button_2 = InlineKeyboardButton::callback("По кинотеатру", callback_data);

    let callback_data = format!("{}{}{}", MenuCode::MainMenu as i32, CD_DELIMETER, ButtonOption::Close as i32);
    let button_3 = InlineKeyboardButton::callback("❌ Закрыть", callback_data);

    let callback_data = format!("{}{}{}", MenuCode::MainMenu as i32, CD_DELIMETER, ButtonOption::Up as i32);
    let button_4 = InlineKeyboardButton::callback("️Наверх ⬆", callback_data);

    InlineKeyboardMarkup::new(vec![vec![button_1, button_2], vec![button_3, button_4]])
}

// Меню кнопок с выбором кинотеатра
// |  Березка   |  Вымпел  |
// |   Искра    |  Космос  |
// |         Сатурн        |
// | ❌ Закрыть | Наверх ⬆ |
pub fn keyboard_cinemas(cinemas: Vec<Cinema>) -> InlineKeyboardMarkup {
    let buttons: Vec<InlineKeyboardButton> = cinemas
        .iter()
        .map(|cinema| {
            let callback_data = format!("{}{}{}", MenuCode::Cinemas as i32, CD_DELIMETER, cinema.id);

            InlineKeyboardButton::callback(cinema.name.to_owned(), callback_data)
        })
        .collect();

    // Группируем кнопки кинотеатров
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = buttons.chunks(2).map(|buttons_row| buttons_row.to_vec()).collect();

    let callback_data = format!("{}{}{}", MenuCode::Cinemas as i32, CD_DELIMETER, ButtonOption::Close as i32);
    let button_1 = InlineKeyboardButton::callback("❌ Закрыть", callback_data);

    let callback_data = format!("{}{}{}", MenuCode::Cinemas as i32, CD_DELIMETER, ButtonOption::Up as i32);
    let button_2 = InlineKeyboardButton::callback("️Наверх ⬆", callback_data);

    // Добавляем кнопки управления
    keyboard.push(vec![button_1, button_2]);

    InlineKeyboardMarkup::new(keyboard)
}

// Меню кнопок с выбором фильма
// | Форест Гамп (1994) |
// | Шоу Трумана (1998) |
// | Леон (1994)        |
// | ⬅️ | ➡ | 1 из 9 | ⬆ |
pub fn keyboard_movielist(movies: Vec<MovieShort>, menu_code: MenuCode, page: i64, total_pages: i64) -> InlineKeyboardMarkup {
    let buttons: Vec<InlineKeyboardButton> = movies
        .iter()
        // .flat_map(|m| {
        .map(|m| {
            let callback_data_1 = format!("{}{}{}", menu_code as i32, CD_DELIMETER, m.id);
            // let callback_data_2 = format!("{}{}{}", menu_code as i32, CD_DELIMETER, m.id);
            //  🗓
            // vec![
            InlineKeyboardButton::callback(&m.title, callback_data_1)
            // InlineKeyboardButton::callback(" 🗓", callback_data_2),
            // ]
        })
        .collect();

    // Группируем кнопки кинотеатров
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = buttons.chunks(1).map(|buttons_row| buttons_row.to_vec()).collect();

    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Back as i32);
    let button_1 = InlineKeyboardButton::callback("⬅️", callback_data);

    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Forward as i32);
    let button_2 = InlineKeyboardButton::callback("➡️", callback_data);

    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Pages as i32);
    let button_3 = InlineKeyboardButton::callback(format!("{} из {}", page, total_pages), callback_data);

    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Up as i32);
    let button_4 = InlineKeyboardButton::callback("️⬆", callback_data);

    // Добавляем кнопки управления
    keyboard.push(vec![button_1, button_2, button_3, button_4]);

    InlineKeyboardMarkup::new(keyboard)
}

// Меню кнопок для варианта 'Все фильмы', когда нету доступных фильмов в прокате
// | Ок 😔 |
// pub fn keyboard_ok(menu_code: MenuCode) -> InlineKeyboardMarkup {
//     let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Close as i32);
//     let button_1 = InlineKeyboardButton::callback("😔 Ок", callback_data);
//     InlineKeyboardMarkup::new(vec![vec![button_1]])
// }

// Меню кнопок для варианта 'По кинотеатру', когда у выбранного кинотеатра нету доступных фильмов в прокате
// или когда нету доступных кинотеатров для показа
// | Ок 😔 | ⬆️ Наверх |
pub fn keyboard_ok_or_up(menu_code: MenuCode) -> InlineKeyboardMarkup {
    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Close as i32);
    let button_1 = InlineKeyboardButton::callback("😔 Ок", callback_data);

    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Up as i32);
    let button_2 = InlineKeyboardButton::callback("️Наверх ⬆", callback_data);
    InlineKeyboardMarkup::new(vec![vec![button_1, button_2]])
}

// TODO
// Ок | [Кинопоиск (link)] | Москино 🗓
pub fn keybord_movie_links(href_mk: Option<&str>, href_kp: Option<&str>, menu_code: MenuCode) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<InlineKeyboardButton> = vec![];

    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Close as i32);
    let button_1 = InlineKeyboardButton::callback("🌝 Ок", callback_data);
    keyboard.push(button_1);

    let callback_data = format!("{}{}{}", menu_code as i32, CD_DELIMETER, ButtonOption::Sessions as i32);
    let button_2 = InlineKeyboardButton::callback("🗓", callback_data);
    keyboard.push(button_2);

    if let Some(mk) = href_mk {
        let parsed = Url::parse(mk).unwrap();
        let button_3 = InlineKeyboardButton::url("Москино", parsed);
        keyboard.push(button_3);
    }

    if let Some(kp) = href_kp {
        let parsed = Url::parse(kp).unwrap();
        let button_4 = InlineKeyboardButton::url("Кинопоиск", parsed);
        keyboard.push(button_4);
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> = keyboard.chunks(2).map(|b| b.to_vec()).collect();
    InlineKeyboardMarkup::new(keyboard)
}
