use super::*;

// тут пока храним непонятную хрень

// bot.edit_message_text(chat.id, id, "⏳").send().await?;
// tokio::time::sleep(Duration::from_secs(1)).await;

pub fn create_movie_description(movie: &Movie) -> String {
    format!(
        "{}{}{}{}{}{}\n",
        movie.title,
        movie.year.map_or("".to_string(), |year| format!("\n\nГод: {}", year)),
        movie.genre.as_ref().map_or("".to_string(), |genre| format!("\nЖанр: {}", genre)),
        movie
            .director
            .as_ref()
            .map_or("".to_string(), |director| format!("\nРежиссер: {}", director)),
        "\n",
        movie
            .description
            .as_ref()
            .map_or("".to_string(), |description| format!("\n{}", description)),
    )
}

pub fn parse_callback_data(callback_data: &str) -> Res<(MenuCode, String)> {
    let mut parts = callback_data.split(CD_DELIMETER);
    match (parts.next(), parts.next()) {
        (Some(a), Some(b)) => {
            //
            let menu_code = match a.parse::<i32>() {
                Ok(menu_code) => match MenuCode::try_from(menu_code) {
                    Ok(code) => code,
                    Err(_) => {
                        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "Invalid MenuCode number")));
                    }
                },
                Err(_) => {
                    return Err(Box::new(io::Error::new(io::ErrorKind::Other, "Failed to parse MenuCode")));
                }
            };
            Ok((menu_code, b.to_string()))
        }
        _ => {
            //
            Err(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid callback_data format: [{}]", callback_data),
            )))
        }
    }
}
