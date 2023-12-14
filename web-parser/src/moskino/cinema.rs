use super::*;
use movie::MoskinoMovie;

lazy_static! {
    static ref STEP_SELECTOR: Selector = Selector::parse(".step").unwrap();
    static ref PLACE_SELECTOR: Selector = Selector::parse(".place-name").unwrap();
}

#[derive(Default)]
pub struct MoskinoCinema {
    pub name: String,
    pub movies: Vec<MoskinoMovie>,
}

impl MoskinoCinema {
    async fn from_node(node: ElementRef<'_>) -> CustomResult<MoskinoCinema> {
        match parse_text(&node, &PLACE_SELECTOR) {
            Some(text) => {
                info!("Cinema: [{text}]");
                let cinema = MoskinoCinema {
                    name: text,
                    movies: MoskinoMovie::parse_vec(node).await,
                };

                Ok(cinema)
            }
            None => {
                let emsg = "Couldn't parse cinema".to_string();
                Err(Box::new(io::Error::new(io::ErrorKind::NotFound, emsg)))
            }
        }
    }

    pub async fn parse_vec(node: ElementRef<'_>) -> Vec<MoskinoCinema> {
        let mut cinemas: Vec<MoskinoCinema> = vec![];

        for step_element in node.select(&STEP_SELECTOR) {
            match MoskinoCinema::from_node(step_element).await {
                Ok(cinema) => {
                    cinemas.push(cinema);
                }
                Err(e) => {
                    error!("Error: {:?}", e);
                    continue;
                }
            }
        }
        cinemas
    }
}

impl fmt::Debug for MoskinoCinema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n{:?}", self.name, self.movies,)
    }
}
