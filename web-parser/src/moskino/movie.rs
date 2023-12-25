use super::*;
use std::hash::{Hash, Hasher};

static URL_KINOPOISK_SEARCH: &str = "https://www.kinopoisk.ru/index.php?kp_query=";
static URL_KINOPOISK: &str = "https://www.kinopoisk.ru/";
static URL_MOSKINO_MOVIES: &str = "https://mos-kino.ru/film/";
static URL_MOSKINO: &str = "https://mos-kino.ru";

lazy_static! {
    static ref R_AGE: Regex = Regex::new(r"(\d+)").unwrap();
    // static ref SCHEDULE_SELECTOR: Selector = Selector::parse(".schedule-item").unwrap();
    static ref TITLE_SELECTOR: Selector = Selector::parse(".title").unwrap();
    static ref SMALL_SELECTOR: Selector = Selector::parse("small").unwrap();
    static ref KP_SELECTOR: Selector = Selector::parse(".name").unwrap();
    static ref KP_A_SELECTOR: Selector = Selector::parse("p.name > a").expect("Failed to parse selector for <a>");
    static ref KP_SPAN_SELECTOR: Selector = Selector::parse("p.name > span.year").expect("Failed to parse selector for <span>");
    static ref MOSKINO_MOVIES: Selector = Selector::parse(".item.toh_paging_item a.movie-item .title").unwrap();
    static ref MOSKINO_MOVIE_GENRE: Selector = Selector::parse(".info-wrapper p").unwrap();
    static ref MOSKINO_MOVIE_INFO: Selector = Selector::parse(".info-wrapper small").unwrap();
    static ref MOSKINO_MOVIE_TAGLINE: Selector = Selector::parse(".description").unwrap();
    static ref MOSKINO_MOVIE_DIRECTOR: Selector = Selector::parse(".info-list .head .lev").unwrap();
    static ref MOSKINO_MOVIE_DESCRIPTION: Selector = Selector::parse(".info-list .text").unwrap();
}

#[derive(Default, Clone, Eq)]
pub struct MoskinoMovie {
    pub title: String,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub director: Option<String>,
    pub description: Option<String>,
    pub href_kinopoisk: Option<String>,
    pub href_moskino: Option<String>,
    pub country: Option<String>,
    pub duration: Option<i32>,
    pub age: Option<i32>,
    pub tagline: Option<String>,
}

impl PartialEq for MoskinoMovie {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
    }
}

impl Hash for MoskinoMovie {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.title.hash(state);
    }
}

impl MoskinoMovie {
    fn draft(title: &str) -> Self {
        MoskinoMovie {
            title: title.to_string(),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    fn draft_with_year(title: &str, year: i32) -> Self {
        MoskinoMovie {
            title: title.to_string(),
            year: Some(year),
            ..Default::default()
        }
    }

    // pub fn from_node(node: ElementRef<'_>, movies: &mut HashSet<Self>) -> CustomResult<()> {
    pub fn from_node(node: ElementRef<'_>) -> CustomResult<MoskinoMovie> {
        // извлекаем название фильма
        match parse_text(&node, &TITLE_SELECTOR) {
            Some(title) => {
                let mut movie = Self::draft(&title);

                // if movies.contains(&movie) {
                // warn!("--- '{}' already in set", movie.title);
                // return Ok(());
                // }

                // parsing year
                if let Some(raw_info) = parse_text(&node, &SMALL_SELECTOR) {
                    match Self::parse_year(&raw_info) {
                        Ok(year) => {
                            movie.year = Some(year);
                        }
                        Err(_) => {
                            warn!("YEAR");
                        }
                    }
                }

                // parsing href moskino
                tokio::task::block_in_place(|| movie.parse_href_moskino())?;

                // parsing other info
                tokio::task::block_in_place(|| movie.parse_movie_info())?;

                // parsing href kinopoisk
                movie.href_kinopoisk = tokio::task::block_in_place(|| Self::parse_href_kinopoisk(&movie))?;

                info!("^^^ '{}'", movie.title);
                // movies.insert(movie);
                // Ok(())
                Ok(movie)
            }
            None => {
                let emsg = "Couldn't parse movie".to_string();
                Err(Box::new(io::Error::new(io::ErrorKind::Other, emsg)))
            }
        }
    }

    fn parse_year(raw: &str) -> CustomResult<i32> {
        let index = raw.find('/');

        if let Some(i) = index {
            Ok(raw[..i].trim().parse::<i32>()?)
        } else {
            let emsg = "Couldn't parse movie".to_string();
            Err(Box::new(io::Error::new(io::ErrorKind::Other, emsg)))
        }
    }

    // search href for a specific movie in the "Movies" tab on Moskino
    fn parse_href_moskino(&mut self) -> CustomResult<()> {
        let html = response_blocking(URL_MOSKINO_MOVIES)?;

        // 1: select all elements based on a given selector
        // 2: transform elements into pairs (element, element text)
        // 3: find the first pair where the element text matches the movie name
        // 4: extract the element from the found pair
        // 5: extract the parent element and the "href" attribute from the element
        match html
            .select(&MOSKINO_MOVIES)
            .filter_map(|el| el.text().next().map(|text| (el, text)))
            .find(|(_, text)| *text == self.title)
            .map(|(el, _)| el)
            .and_then(|el| el.parent_element().and_then(|a| a.attr("href")))
        {
            Some(href) => {
                self.href_moskino = Some(URL_MOSKINO.to_owned() + href);
                Ok(())
            }
            None => {
                warn!("{:?}", self);
                let emsg = format!("'{}': couldn't find a href on moskino website", self.title);
                Err(Box::new(io::Error::new(io::ErrorKind::NotFound, emsg)))
            }
        }
    }

    fn parse_movie_info(&mut self) -> CustomResult<()> {
        let url = self.href_moskino.as_deref().ok_or_else(|| {
            let emsg = format!("'{}': no href to the movie for full parsing", self.title);
            Box::new(io::Error::new(io::ErrorKind::NotFound, emsg))
        })?;

        let html = response_blocking(url)?;
        let node = html.root_element();

        if let Some(genre) = parse_text(&node, &MOSKINO_MOVIE_GENRE) {
            self.genre = Some(genre);
        } else {
            warn!("Genre");
        }

        if let Some(info) = parse_text(&node, &MOSKINO_MOVIE_INFO) {
            self.parse_info(info);
        } else {
            warn!("Info")
        }

        if let Some(tagline) = parse_text(&node, &MOSKINO_MOVIE_TAGLINE) {
            self.tagline = Some(tagline);
        } else {
            warn!("Tagline");
        }

        if let Some(director) = parse_text(&node, &MOSKINO_MOVIE_DIRECTOR) {
            self.director = Some(director);
        } else {
            warn!("Director");
        }

        if let Some(description) = parse_text(&node, &MOSKINO_MOVIE_DESCRIPTION) {
            self.description = Some(description);
        } else {
            warn!("Description");
        }

        Ok(())
    }

    fn parse_info(&mut self, raw: String) {
        let splitted: Vec<String> = raw.split("/ ").map(|s| s.to_string()).collect();

        if !splitted[0].trim().is_empty() {
            self.country = Some(splitted[0].trim().to_string());
        } else {
            warn!("Country");
        }

        if !splitted[1].trim().is_empty() {
            if let Ok(year) = splitted[1].trim().parse::<i32>() {
                self.year = Some(year);
            } else {
                warn!("Year");
            }
        } else {
            warn!("Year");
        }

        if !splitted[2].trim().is_empty() {
            let duration: String = splitted[2].chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(duration) = duration.trim().parse::<i32>() {
                self.duration = Some(duration);
            } else {
                warn!("Duration");
            }
        } else {
            warn!("Duration");
        }

        if !splitted[3].trim().is_empty() {
            if let Some(num) = parse_num_with_regex(splitted[3].trim(), &R_AGE) {
                self.age = Some(num);
            } else {
                warn!("Age Rating");
            }
        } else {
            warn!("Age Rating");
        }
    }

    fn parse_href_kinopoisk(movie: &MoskinoMovie) -> CustomResult<Option<String>> {
        // Kinopoisk query link
        let url = Self::create_url_to_search(&movie.title, movie.year);

        let link = MoskinoMovie::kinopoisk_get_link(url, movie)?;

        match link {
            Some(link) => Ok(Some(link)),
            None => {
                // trying without year (it may be incorrect) for the movie (only by name)
                if movie.year.is_some() {
                    let mut movie = movie.clone();
                    movie.year = None;

                    let url = Self::create_url_to_search(&movie.title, None);
                    MoskinoMovie::kinopoisk_get_link(url, &movie)
                } else {
                    warn!("KP href");
                    Ok(None)
                }
            }
        }
    }

    fn kinopoisk_get_link(url: String, movie: &MoskinoMovie) -> CustomResult<Option<String>> {
        let html = response_blocking(&url)?;

        let a = html.select(&KP_A_SELECTOR).next();

        let link = match a {
            Some(a) => {
                let link = a.value().attr("href");
                let name = a.text().next();

                if link.is_some() && name.eq(&Some(&movie.title)) {
                    link
                } else {
                    return Ok(None);
                }
            }
            None => {
                return Ok(None);
            }
        };

        let link = match link {
            Some(link) => String::from(link),
            None => {
                return Ok(None);
            }
        };

        // if a year is provided, then we check it against the data specified on the website
        if let Some(orig_year) = movie.year {
            let span = html.select(&KP_SPAN_SELECTOR).next();
            match span {
                Some(span) => {
                    let year = span.text().next();

                    if let Some(year) = year {
                        match year.parse::<i32>() {
                            Ok(year) => {
                                if year != orig_year {
                                    return Ok(None);
                                }
                            }
                            Err(_) => {
                                // TODO
                                return Ok(None);
                            }
                        }
                    }
                }
                None => {
                    return Ok(None);
                }
            }
        }

        Ok(Some(format!("{}{}", URL_KINOPOISK, link)))
    }

    // standard URL query in Kinopoisk href
    fn create_url_to_search(name: &str, year: Option<i32>) -> String {
        let query = if let Some(val) = year {
            format!("{} {}", name, val)
        } else {
            String::from(name)
        };
        let query_enc: String = form_urlencoded::byte_serialize(query.as_bytes()).collect();
        format!("{}{}", URL_KINOPOISK_SEARCH, query_enc)
    }
}

impl fmt::Debug for MoskinoMovie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\n\t\
            title: [{}]\n\t\
            year: [{}]\n\t\
            genre: [{}]\n\t\
            country: [{}]\n\t\
            director: [{}]\n\t\
            duration: [{}]\n\t\
            age: [{}]\n\t\
            tagline: [{}]\n\t\
            description: [{}]\n\t\
            href_kinopoisk: [{}]\n\t\
            href_moskino: [{}]",
            self.title,
            self.year.unwrap_or_default(),
            self.genre.as_deref().unwrap_or(""),
            self.country.as_deref().unwrap_or(""),
            self.director.as_deref().unwrap_or(""),
            self.duration.unwrap_or_default(),
            self.age.unwrap_or_default(),
            self.tagline.as_deref().unwrap_or(""),
            self.description.as_deref().unwrap_or(""),
            self.href_kinopoisk.as_deref().unwrap_or(""),
            self.href_moskino.as_deref().unwrap_or(""),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_year() {
        let case1 = "2023 / 88 min / Canada, Germany / 6+";
        let case2 = "2023 / min / / 6+";
        let case3 = "2023 / 132 min / / 18+";
        // let case4 = "/ 110 min / France / 16+";

        MoskinoMovie::parse_year(case1).unwrap();
        MoskinoMovie::parse_year(case2).unwrap();
        MoskinoMovie::parse_year(case3).unwrap();
    }

    #[test]
    fn test_create_url_to_search() {
        let url = MoskinoMovie::create_url_to_search("Бешеные псы", Some(1991));

        assert_eq!(
            url,
            "https://www.kinopoisk.ru/index.php?kp_query=%D0%91%D0%B5%D1%88%D0%B5%D0%BD%D1%8B%D0%B5+%D0%BF%D1%81%D1%8B+1991"
        );
    }

    #[test]
    fn test_kinopoisk() {
        // ok some
        let movie = MoskinoMovie::draft_with_year("Бешеные псы", 1991);
        let result = MoskinoMovie::parse_href_kinopoisk(&movie);

        match result {
            Ok(link) => {
                assert_eq!(link.as_deref(), Some("https://www.kinopoisk.ru//film/394/sr/1/"));
            }
            Err(err) => {
                panic!("Error: {:?}", err);
            }
        }

        // ok none non-existent movie
        let movie = MoskinoMovie::draft_with_year("Non-existent movie blup blip", 1504);
        let result = MoskinoMovie::parse_href_kinopoisk(&movie);

        match result {
            Ok(link) => {
                assert_eq!(link, None);
            }
            Err(err) => {
                panic!("Error: {:?}", err);
            }
        }

        // ok some with wrong year
        let movie = MoskinoMovie::draft_with_year("Бешеные псы", 2023);
        let result = MoskinoMovie::parse_href_kinopoisk(&movie);

        match result {
            Ok(link) => {
                assert_eq!(link.as_deref(), Some("https://www.kinopoisk.ru//film/394/sr/1/"));
            }
            Err(err) => {
                panic!("Error: {:?}", err);
            }
        }

        // ok some without year
        let movie = MoskinoMovie::draft("Олдбой");
        let result = MoskinoMovie::parse_href_kinopoisk(&movie);

        match result {
            Ok(link) => {
                assert_eq!(link.as_deref(), Some("https://www.kinopoisk.ru//film/75871/sr/1/"));
            }
            Err(err) => {
                panic!("Error: {:?}", err);
            }
        }
    }

    // #[test]
    // fn test_moskino_get_link() {
    //     let mut movie = MoskinoMovie::draft_with_year("Олдбой", 2003);
    //     let movie_href = movie.parse_href_moskino();

    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::full_parse(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);

    //     let mut movie = MoskinoMovie::draft_with_year("Великая магия", 2003);
    //     let movie_href = movie.parse_href_moskino();
    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::parse_info(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);

    //     let mut movie = MoskinoMovie::draft_with_year("Укрась прощальное утро цветами обещания", "2023");
    //     let movie_href = movie.parse_href_moskino();
    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::full_parse(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);

    //     let mut movie = MoskinoMovie::draft_with_year("Основной инстинкт", "1992");
    //     let movie_href = :=movie.parse_href_moskino();
    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::full_parse(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);
    // }
}
