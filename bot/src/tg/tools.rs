use super::*;

// bot.edit_message_text(chat.id, id, "⏳").send().await?;
// tokio::time::sleep(Duration::from_secs(1)).await;

pub fn callback_parse(callback_data: &str) -> Res<(MenuCode, String)> {
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
