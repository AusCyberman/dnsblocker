-- This file should undo anything in `up.sql`
DELETE FROM sessions;
DELETE FROM clients;
DELETE FROM domain;
DELETE FROM users;