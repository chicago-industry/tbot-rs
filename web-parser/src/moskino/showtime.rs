use super::*;

lazy_static! {
    static ref SUBITEM_SELECTOR: Selector = Selector::parse(".subitem").unwrap();
    static ref TIME_SELECTOR: Selector = Selector::parse(".time").unwrap();
    static ref PRICE_SELECTOR: Selector = Selector::parse(".price").unwrap();
    static ref R_PRICE: Regex = Regex::new(r"(\d+) \w").unwrap();
}

#[derive(Default, Debug, Clone)]
pub struct MoskinoShowTime {
    pub time: NaiveTime,
    pub price: i32,
}

// fn parse_num_with_regex(text: &str, regex: &Regex) -> Option<i32> {
//     if let Some(captures) = regex.captures(text.trim()) {
//         if let Some(matched) = captures.get(1) {
//             if let Ok(price) = matched.as_str().parse::<i32>() {
//                 return Some(price);
//             }
//         }
//     }
//     None
// }

impl MoskinoShowTime {
    // .time
    // .price
    pub async fn parse(&mut self, node: ElementRef<'_>) -> CustomResult<()> {
        let time = parse_text(&node, &TIME_SELECTOR);
        let price = parse_text(&node, &PRICE_SELECTOR);

        if let (Some(time), Some(price)) = (time, price) {
            match NaiveTime::parse_from_str(&time, "%H:%M") {
                Ok(time) => {
                    self.time = time;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }

            match parse_num_with_regex(&price, &R_PRICE) {
                Some(price) => {
                    self.price = price;
                }
                None => {
                    error!("Price");
                    self.price = 0;
                    // let emsg = "Couldn't parse showtime price".to_string();
                    // return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, emsg)));
                }
            }

            Ok(())
        } else {
            let emsg = "Couldn't parse showtime".to_string();
            Err(Box::new(io::Error::new(io::ErrorKind::NotFound, emsg)))
        }
    }

    // .subitem
    //     .time
    //     .price
    // ...
    // .subitem
    //     .time
    //     .price
    pub async fn parse_vec(schedule: ElementRef<'_>) -> Vec<Self> {
        let mut showtimes = vec![];

        for subitem_element in schedule.select(&SUBITEM_SELECTOR) {
            let mut showtime: MoskinoShowTime = Default::default();

            match showtime.parse(subitem_element).await {
                Ok(_) => {
                    showtimes.push(showtime);
                }
                Err(e) => {
                    error!("Error: {:?}", e);
                    continue;
                }
            }
        }
        showtimes
    }
}
