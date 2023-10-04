-- Your SQL goes here
CREATE TABLE IF NOT EXISTS users (
    id serial NOT NULL PRIMARY KEY,
    name text NOT NULL,
    username text NOT NULL
);
CREATE TABLE domain (
    id serial NOT NULL PRIMARY KEY,
    domain_name text NOT NULL,
    user_id integer NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE TABLE IF NOT EXISTS clients (
    id serial NOT NULL PRIMARY KEY,
    block_all boolean NOT NULL,
    ip text NOT NULL,
    user_id integer NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE TABLE IF NOT EXISTS sessions (
    id serial NOT NULL PRIMARY KEY,
    user_id integer NOT NULL,
    time_left integer,
    end_timestamp timestamp,
    FOREIGN KEY (user_id) REFERENCES users(id)
);