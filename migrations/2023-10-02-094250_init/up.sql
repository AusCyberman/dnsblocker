-- Your SQL goes here
CREATE TABLE IF NOT EXISTS users (
    id integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    name text NOT NULL,
    username text NOT NULL
);
CREATE TABLE domain (
    id integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    domain_name text NOT NULL,
    user_id integer NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE TABLE IF NOT EXISTS clients (
    id integer NOT NULL PRIMARY KEY AUTOINCREMENT,
    block_all boolean NOT NULL,
    ip text NOT NULL,
    user_id integer NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id integer NOT NULL,
    time_left integer,
    end_timestamp timestamp,
    FOREIGN KEY (user_id) REFERENCES users(id)
);