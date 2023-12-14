use super::*;
// use scraper::html;
use showtime::MoskinoShowTime;

static URL_KINOPOISK_SEARCH: &str = "https://www.kinopoisk.ru/index.php?kp_query=";
static URL_KINOPOISK: &str = "https://www.kinopoisk.ru/";
// static URL_MOSKINO_SCHEDULE: &str = "https://mos-kino.ru/schedule/";
static URL_MOSKINO_MOVIES: &str = "https://mos-kino.ru/film/";
static URL_MOSKINO: &str = "https://mos-kino.ru";

lazy_static! {
    static ref R_AGE: Regex = Regex::new(r"(\d+)").unwrap();

    static ref TITLE_SELECTOR: Selector = Selector::parse(".title").unwrap();
    static ref SCHEDULE_SELECTOR: Selector = Selector::parse(".schedule-item").unwrap();

    static ref KP_SELECTOR: Selector = Selector::parse(".name").unwrap();
    static ref KP_A_SELECTOR: Selector = Selector::parse("p.name > a").expect("Failed to parse selector for <a>");
    static ref KP_SPAN_SELECTOR: Selector = Selector::parse("p.name > span.year").expect("Failed to parse selector for <span>");

    static ref SMALL_SELECTOR: Selector = Selector::parse("small").unwrap();

    // для поиска конкретного фильма во вкладке фильмы на Москино
    static ref MOSKINO_MOVIES: Selector = Selector::parse(".item.toh_paging_item a.movie-item .title").unwrap();

    // <div class="info-wrapper">
    // <p>Фэнтези / Анимация / Мелодрама</p>
    // <small>Япония / 2018 / 115 мин / 12+</small>
    // </div>
    //
    // <p>Фэнтези / Анимация / Мелодрама</p>
    static ref MOSKINO_MOVIE_GENRE: Selector = Selector::parse(".info-wrapper p").unwrap();
    // <small>Япония / 2018 / 115 мин / 12+</small>
    static ref MOSKINO_MOVIE_INFO: Selector = Selector::parse(".info-wrapper small").unwrap();
    // <div class="description">Фантастическое аниме про любовь бессмертной красавицы и ее воспитанника </div>
    static ref MOSKINO_MOVIE_TAGLINE: Selector = Selector::parse(".description").unwrap();

    // <div class="info-list">
    // 	<div class="step">
    // 		<div class="head">
    // 			<div class="val">Режиссер</div>
    // 			<div class="lev">Мари Окада, Тосия Синохара</div>
    // 		</div>
    // 		<div class="text">
    // 			Красавица Макия — из рода бессмертных. На протяжении веков многие армии пытались
    // 			захватить её народ, чтобы завладеть секретом вечной жизни. И вот теперь, когда город
    // 			разрушен, девушка прячется в лесу. Здесь она встречает потерявшего родителей маленького
    // 			мальчика Эриала и начинает о нём заботиться... Проходят годы, мальчик превращается в
    // 			прекрасного юношу, и у беглецов возникают чувства друг к другу. Но Макия понимает, что
    // 			Эриал — простой человек, а значит, он смертен... Удастся ли им спасти свою любовь?<br>
    // 			<br>
    // 			Хронометраж фильма без показа рекламных роликов - 115 мин.
    // 		</div>
    // 	</div>
    // 	<div class="step">
    // 		<div class="head">
    // 			<div class="label">В ролях</div>
    // 		</div>
    // 		<div class="text">
    // 			Манака Ивами, Мию Ирино </div>
    // 	</div>
    // </div>
    //
    // <div class="lev">Мари Окада, Тосия Синохара</div>
    static ref MOSKINO_MOVIE_DIRECTOR: Selector = Selector::parse(".info-list .head .lev").unwrap();
    // <div class="text">...</div>
    static ref MOSKINO_MOVIE_DESCRIPTION: Selector = Selector::parse(".info-list .text").unwrap();

}

#[derive(Default, Clone)]
pub struct MoskinoMovie {
    pub title: String,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub director: Option<String>,
    pub description: Option<String>,
    pub link_kinopoisk: Option<String>,
    pub link_moskino: Option<String>,
    pub country: Option<String>,
    pub duration: Option<i32>,
    pub age: Option<i32>,
    pub tagline: Option<String>,
    pub showings: Vec<MoskinoShowTime>,
}

impl MoskinoMovie {
    pub fn draft(title: &str) -> Self {
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

    async fn from_node(node: ElementRef<'_>) -> CustomResult<MoskinoMovie> {
        // извлекаем название фильма
        match parse_text(&node, &TITLE_SELECTOR) {
            Some(name) => {
                info!("Movie: [{name}]");
                let mut movie = MoskinoMovie::draft(&name);

                // извлекаем год
                if let Some(raw_info) = parse_text(&node, &SMALL_SELECTOR) {
                    movie.set_year_from_raw(&raw_info);
                }

                // извлекаем ссылку на фильм
                movie.set_href_moskino().await?;

                // извлекаем информацию по фильму
                movie.full_parse().await?;

                // link for kinopoisk
                movie.link_kinopoisk = Self::kinopoisk_find(&movie).await?;

                movie.showings = MoskinoShowTime::parse_vec(node).await;

                Ok(movie)
            }
            None => {
                let emsg = "Couldn't parse movie".to_string();
                Err(Box::new(io::Error::new(io::ErrorKind::NotFound, emsg)))
            }
        }
    }

    pub async fn parse_vec(node: ElementRef<'_>) -> Vec<MoskinoMovie> {
        let mut movies: Vec<MoskinoMovie> = vec![];

        for step_element in node.select(&SCHEDULE_SELECTOR) {
            match MoskinoMovie::from_node(step_element).await {
                Ok(movie) => movies.push(movie),
                Err(e) => {
                    error!("Error: {:?}", e);
                    continue;
                }
            }
        }
        movies
    }

    // <div class="item toh_paging_item" style="">
    // 	<a href="/film/mertvets/" class="movie-item">
    // 		<span class="title">Мертвец</span>
    // 		<span class="contet">
    // 			<span class="d-flex a-center">
    // 				<span class="info">
    // 					<span class="p">США, Япония, Германия</span>
    // 					<span class="p">1995 / 121 мин / 18+</span>
    // 				</span>
    // 		    </span>
    // 		    <span class="img">
    // 				<img src="/upload/resize_cache/iblock/b62/otafc2hj24r44215l1h8z78do3jp4qph/500_500_1/220605180603852.jpg" alt="">
    // 			</span>
    // 		</span>
    // 	</a>
    // </div>
    pub async fn set_href_moskino(&mut self) -> CustomResult<()> {
        let html = response_get(URL_MOSKINO_MOVIES).await?;

        // 1: выбираем все элементы по заданному селектору
        // 2: преобразуем элементы в пары (элемент, текст элемента)
        // 3: ищем первую пару, где текст элемента совпадает с именем фильма
        // 4: извлекаем элемент из найденной пары
        // 5: извлекаем родительский элемент и атрибут "href" из элемента
        match html
            .select(&MOSKINO_MOVIES)
            .filter_map(|el| el.text().next().map(|text| (el, text)))
            .find(|(_, text)| text == &self.title)
            .map(|(el, _)| el)
            .and_then(|el| el.parent_element().and_then(|a| a.attr("href")))
        {
            Some(href) => {
                self.link_moskino = Some(URL_MOSKINO.to_owned() + href);
                Ok(())
            }
            None => {
                let emsg = format!("Movie [{}]: Couldn't find a href to the movie for full parsing", self.title);
                Err(Box::new(io::Error::new(io::ErrorKind::NotFound, emsg)))
            }
        }
    }

    async fn full_parse(&mut self) -> CustomResult<()> {
        let url = self.link_moskino.as_deref().ok_or_else(|| {
            let emsg = format!("Movie [{}]: Couldn't find a href to the movie for full parsing", self.title);
            Box::new(io::Error::new(io::ErrorKind::NotFound, emsg))
        })?;

        let html = response_get(url).await?;
        let node = html.root_element();

        if let Some(genre) = parse_text(&node, &MOSKINO_MOVIE_GENRE) {
            self.genre = Some(genre);
        } else {
            warn!("Genre");
        }

        if let Some(info) = parse_text(&node, &MOSKINO_MOVIE_INFO) {
            self.set_info_from_raw(info);
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

    // Извлечение информации о фильме (год, продолжительность, страна, возрастной рейтинг)
    // "2023 / 88 мин / Канада, Германия / 6+"
    // "2023 / мин / / 6+"
    // "2023 / 132 мин / / 18+"
    // "/ 110 мин / Франция, Швеция / 16+"
    fn set_info_from_raw(&mut self, raw: String) {
        let splitted: Vec<String> = raw.split("/ ").map(|s| s.to_string()).collect();

        // Country
        if !splitted[0].trim().is_empty() {
            self.country = Some(splitted[0].trim().to_string());
        } else {
            warn!("Country");
        }

        // Year
        if !splitted[1].trim().is_empty() {
            if let Ok(year) = splitted[1].trim().parse::<i32>() {
                self.year = Some(year);
            } else {
                warn!("Year");
            }
        } else {
            warn!("Year");
        }

        // Duration
        if !splitted[2].trim().is_empty() {
            // Извлекаем числа из строки
            let duration: String = splitted[2].chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(duration) = duration.trim().parse::<i32>() {
                self.duration = Some(duration);
            } else {
                warn!("Duration");
            }
        } else {
            warn!("Duration");
        }

        // Age rating
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

    // Извлечение года выпуска фильма
    // "2023 / 88 мин / Канада, Германия / 6+"
    // "2023 / мин / / 6+"
    // "2023 / 132 мин / / 18+"
    // "/ 110 мин / Франция, Швеция / 16+"
    fn set_year_from_raw(&mut self, raw: &str) {
        if let Some(index) = raw.find('/') {
            if let Ok(year) = raw[..index].trim().parse::<i32>() {
                self.year = Some(year);
            }
        }
        warn!("Year");
    }

    async fn kinopoisk_find(movie: &MoskinoMovie) -> CustomResult<Option<String>> {
        // kinopoisk query link
        let url = Self::create_url_to_search(&movie.title, movie.year);

        // println!("URL: {}", url);

        let link = MoskinoMovie::kinopoisk_get_link(url, movie).await?;

        match link {
            Some(link) => Ok(Some(link)),
            None => {
                // trying without year (it may be incorrect) for the movie (only by name)
                if movie.year.is_some() {
                    let url = Self::create_url_to_search(&movie.title, None);
                    MoskinoMovie::kinopoisk_get_link(url, movie).await
                } else {
                    warn!("KP link");
                    println!("KP link");
                    Ok(None)
                }
            }
        }
    }

    // кирилл, тебе не стыдно такой код писать после года разработки?
    // главное, это работает)))0
    async fn kinopoisk_get_link(url: String, movie: &MoskinoMovie) -> CustomResult<Option<String>> {
        let html = response_get(&url).await?;

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

        // если передали год, то проверяем и его с данными, указанными на сайте
        if let Some(orig_year) = movie.year {
            //
            let span = html.select(&KP_SPAN_SELECTOR).next();
            match span {
                Some(span) => {
                    let year = span.text().next();

                    if let Some(year) = year {
                        match year.parse::<i32>() {
                            //
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

                    //
                }
                None => {
                    return Ok(None);
                }
            }
        }

        Ok(Some(format!("{}{}", URL_KINOPOISK, link)))
    }

    // kinopoisk link with query in URL standard
    fn create_url_to_search(name: &str, year: Option<i32>) -> String {
        let query = if let Some(val) = year { format!("{} {}", name, val) } else { String::from(name) };
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
            href_moskino: [{}]\n\t\
            showtime: [{:?}]",
            self.title,
            self.year.unwrap_or_default(),
            self.genre.as_deref().unwrap_or(""),
            self.country.as_deref().unwrap_or(""),
            self.director.as_deref().unwrap_or(""),
            self.duration.unwrap_or_default(),
            self.age.unwrap_or_default(),
            self.tagline.as_deref().unwrap_or(""),
            self.description.as_deref().unwrap_or(""),
            self.link_kinopoisk.as_deref().unwrap_or(""),
            self.link_moskino.as_deref().unwrap_or(""),
            self.showings,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_macros)]
    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_create_url_to_search() {
        let url = MoskinoMovie::create_url_to_search("Бешеные псы", Some(1991));

        assert_eq!(
            url,
            "https://www.kinopoisk.ru/index.php?kp_query=%D0%91%D0%B5%D1%88%D0%B5%D0%BD%D1%8B%D0%B5+%D0%BF%D1%81%D1%8B+1991"
        );

        // let url = MoskinoMovie::create_url_to_search("Манюня: новогодние приключения", Some(2023));
        // println!("URL= {}", url);
    }

    #[test]
    fn test_kinopoisk() {
        // // ok some
        // let movie = MoskinoMovie::draft_with_year("Бешеные псы", 1991);
        // let result = aw!(MoskinoMovie::kinopoisk_find(&movie));

        // match result {
        //     Ok(link) => {
        //         assert_eq!(link.as_deref(), Some("https://www.kinopoisk.ru//film/394/sr/1/"));
        //     }
        //     Err(err) => {
        //         panic!("Error: {:?}", err);
        //     }
        // }

        // // ok none non-existent movie
        // let movie = MoskinoMovie::draft_with_year("Non-existent movie blup blip", 1504);
        // let result = aw!(MoskinoMovie::kinopoisk_find(&movie));

        // match result {
        //     Ok(link) => {
        //         assert_eq!(link, None);
        //     }
        //     Err(err) => {
        //         panic!("Error: {:?}", err);
        //     }
        // }

        // // ok some with wrong year
        // let movie = MoskinoMovie::draft_with_year("Бешеные псы", 2023);
        // let result = aw!(MoskinoMovie::kinopoisk_find(&movie));

        // match result {
        //     Ok(link) => {
        //         assert_eq!(link.as_deref(), Some("https://www.kinopoisk.ru//film/394/sr/1/"));
        //     }
        //     Err(err) => {
        //         panic!("Error: {:?}", err);
        //     }
        // }

        // // ok some without year
        // let movie = MoskinoMovie::draft("Олдбой");
        // let result = aw!(MoskinoMovie::kinopoisk_find(&movie));

        // match result {
        //     Ok(link) => {
        //         assert_eq!(link.as_deref(), Some("https://www.kinopoisk.ru//film/75871/sr/1/"));
        //     }
        //     Err(err) => {
        //         panic!("Error: {:?}", err);
        //     }
        // }

        let movie = MoskinoMovie::draft("Манюня: Новогодние приключения");
        let result = aw!(MoskinoMovie::kinopoisk_find(&movie));
        println!("sukaaa {:?}", result);

        let movie = MoskinoMovie::draft("Лунатики");
        let result = aw!(MoskinoMovie::kinopoisk_find(&movie));
        println!("sukaaa {:?}", result);
        // match result {
        //     Ok(link) => {
        //         assert_eq!(link.as_deref(), Some("https://www.kinopoisk.ru/film/5253703/"));
        //     }
        //     Err(err) => {
        //         panic!("Error: {:?}", err);
        //     }
        // }
    }

    // #[test]
    // fn test_moskino_get_link() {
    //     let mut movie = MoskinoMovie::draft_with_year("Олдбой", 2003);
    //     let movie_href = aw!(MoskinoMovie::moskino_get_link(&movie));
    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::full_parse(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);

    //     let mut movie = MoskinoMovie::draft("Великая магия", 2003);
    //     let movie_href = aw!(MoskinoMovie::moskino_get_link(&movie));
    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::full_parse(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);

    //     let mut movie = MoskinoMovie::draft("Укрась прощальное утро цветами обещания", "2023");
    //     let movie_href = aw!(MoskinoMovie::moskino_get_link(&movie));
    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::full_parse(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);

    //     let mut movie = MoskinoMovie::draft("Основной инстинкт", "1992");
    //     let movie_href = aw!(MoskinoMovie::moskino_get_link(&movie));
    //     println!("{:?}", movie_href);
    //     let info = aw!(MoskinoMovie::full_parse(&movie, movie_href.unwrap()));
    //     println!("info: [{:?}]", info);
    // }
}
