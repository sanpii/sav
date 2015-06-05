CREATE TABLE expense (
    id SERIAL PRIMARY KEY,
    created TIMESTAMP WITHOUT TIME ZONE DEFAULT now() NOT NULL,
    serial CHARACTER VARYING,
    name CHARACTER VARYING NOT NULL,
    url CHARACTER VARYING,
    shop CHARACTER VARYING NOT NULL,
    warranty INTERVAL NOT NULL,
    price REAL NOT NULL,
    trashed BOOLEAN NOT NULL DEFAULT false
);