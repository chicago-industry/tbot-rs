CREATE SCHEMA IF NOT EXISTS moskino;

CREATE TABLE moskino.cinemas (
    cinema_id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    address VARCHAR(255),
    metro VARCHAR(255),
    is_active BOOLEAN
);

CREATE TABLE moskino.movies (
    movie_id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    year INT,
    genre VARCHAR(100),
    country VARCHAR(100),
    duration INT,
    age INT,
    director VARCHAR(100),
    tagline TEXT,
    description TEXT,
    href_moskino VARCHAR(100),
    href_kinopoisk VARCHAR(100),

    UNIQUE (title, year)
);
-- CONSTRAINT unique_title_year UNIQUE (title, year)

CREATE TABLE moskino.sessions (
    session_id SERIAL PRIMARY KEY,
    movie_id INTEGER REFERENCES moskino.movies(movie_id),
    cinema_id INTEGER REFERENCES moskino.cinemas(cinema_id),
    showtime TIME NOT NULL,
    showdate DATE NOT NULL,
    price INTEGER NOT NULL,

    UNIQUE (cinema_id, movie_id, showdate, showtime, price)
);

INSERT INTO moskino.movies (
    title,
    year,
    genre,
    country,
    duration,
    age,
    director,
    tagline,
    description,
    href_moskino,
    href_kinopoisk
) VALUES (
    'Test Movie',
    2023,
    'Action',
    'United States',
    120,
    16,
    'Test Director',
    'An exciting test movie',
    'This is a description of the test movie.',
    'http://example.com/moskino/test-movie',
    'http://example.com/kinopoisk/test-movie'
);