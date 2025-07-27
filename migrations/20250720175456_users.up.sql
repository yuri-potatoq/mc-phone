CREATE TABLE IF NOT EXISTS rcon_users (
    ID INTEGER PRIMARY KEY,
    game_nick TEXT NOT NULL,
    password TEXT NOT NULL,
    UNIQUE(game_nick, password)
);