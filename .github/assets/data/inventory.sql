-- Seed for the database.gif tape.  Kept as SQL rather than a committed .sqlite so
-- the tape can rebuild it from scratch on every run.
CREATE TABLE parts (
  id       INTEGER PRIMARY KEY,
  name     TEXT NOT NULL,
  qty      INTEGER DEFAULT 0,
  price    REAL,
  supplier TEXT
);

CREATE INDEX parts_name ON parts(name);

INSERT INTO parts (name, qty, price, supplier) VALUES
  ('bolt M6',      120, 0.35, 'Acme'),
  ('nut M6',       340, 0.12, 'Acme'),
  ('washer M6',     90, 0.05, 'Globex'),
  ('bearing 608',   12, 14.50, NULL),
  ('spring 20mm',   64, 1.80, 'Globex');
