use super::*;

lazy_static! {
    static ref PLACE_SELECTOR: Selector = Selector::parse(".place-name").unwrap();
}

#[derive(Default, Debug)]
pub struct MoskinoCinema {
    pub name: String,
}

impl MoskinoCinema {
    pub fn from_node(node: ElementRef<'_>) -> CustomResult<MoskinoCinema> {
        match parse_text(&node, &PLACE_SELECTOR) {
            Some(text) => Ok(MoskinoCinema { name: text }),
            None => {
                let emsg = "Couldn't parse cinema".to_string();
                Err(Box::new(io::Error::new(io::ErrorKind::Other, emsg)))
            }
        }
    }
}
