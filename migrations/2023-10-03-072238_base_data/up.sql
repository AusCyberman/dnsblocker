-- Your SQL goes here
INSERT INTO users (name, username)
VALUES ("Ivy", "ivy");
INSERT INTO domain (domain_name, user_id)
VALUES ("snapchat.com", 1),
    ("instagram.com", 1),
    ("discord.com", 1);
INSERT INTO clients (block_all, ip, user_id)
VALUES (true, "127.0.0.1", 1);
INSERT INTO sessions (user_id, end_timestamp)
VALUES (
        1,
        datetime('now', '2 minute')
    );