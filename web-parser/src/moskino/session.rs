use super::*;

lazy_static! {
    static ref TIME_SELECTOR: Selector = Selector::parse(".time").unwrap();
    static ref PRICE_SELECTOR: Selector = Selector::parse(".price").unwrap();
    static ref R_PRICE: Regex = Regex::new(r"(\d+) \w").unwrap();
}

#[derive(Default, Debug, Clone)]
pub struct MoskinoSession {
    pub time: NaiveTime,
    pub price: i32,
}

impl MoskinoSession {
    // .time
    // .price
    pub fn from_node(node: ElementRef<'_>) -> CustomResult<Self> {
        let mut session = MoskinoSession::default();

        let time = parse_text(&node, &TIME_SELECTOR);
        let price = parse_text(&node, &PRICE_SELECTOR);

        if let (Some(time), Some(price)) = (time, price) {
            match NaiveTime::parse_from_str(&time, "%H:%M") {
                Ok(time) => {
                    session.time = time;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }

            match parse_num_with_regex(&price, &R_PRICE) {
                Some(price) => {
                    session.price = price;
                }
                None => {
                    // for free sessions
                    error!("Price");
                    session.price = 0;
                }
            }

            Ok(session)
        } else {
            let emsg = "Couldn't parse showtime".to_string();
            Err(Box::new(io::Error::new(io::ErrorKind::NotFound, emsg)))
        }
    }
}
