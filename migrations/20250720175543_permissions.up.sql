CREATE TABLE IF NOT EXISTS users_permissions (
    ID INTEGER PRIMARY KEY,
    user_id INTEGER,
    command TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES rcon_users(ID)
);