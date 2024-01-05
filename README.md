# tbot-rs
for educational and fun

## .env file
```
# db stuff
POSTGRES_USER=<USER>
POSTGRES_PASSWORD=<PASSWORD>
POSTGRES_DB_NAME="postgres"
POSTGRES_PORT="5432"
POSTGRES_DATA_LOCATION=<LOCATE ON HOST MACHINE>

DATABASE_URL="postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@db:$POSTGRES_PORT/$POSTGRES_DB_NAME"
DATABASE_MAX_CONNECTIONS=10

# tg token
TELOXIDE_TOKEN=<TELEGRAM BOT TOKEN>

# bot app settings
MOSKINO_BOT_ITEMS_PER_PAGE=4
```

## run
```
# to get data
WEB_PARSER_ARG=today docker compose up web-parser
WEB_PARSER_ARG=tommorow docker compose up web-parser
WEB_PARSER_ARG=aftertommorow docker compose up web-parser

# to run bot
docker compose up -d bot
```
