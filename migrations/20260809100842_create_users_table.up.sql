-- Add up migration script here
CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  username TEXT NOT NULL UNIQUE,
  email TEXT NOT NULL UNIQUE,
  password VARCHAR(255) NOT NULL,
  displayName TEXT,
  about TEXT
)
