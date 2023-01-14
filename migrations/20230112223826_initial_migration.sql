-- Add migration script here
CREATE TABLE users (
    discord_id     TEXT    NOT NULL,
    hoyo_cookie_id INTEGER NOT NULL,
    genshin_uid    TEXT    NOT NULL,
    PRIMARY KEY (
        discord_id,
        genshin_uid
    ),
    FOREIGN KEY (
        genshin_uid
    )
    REFERENCES config (genshin_uid),
    FOREIGN KEY (
        hoyo_cookie_id
    )
    REFERENCES hoyo_cookie (cookie_id) 
);

CREATE TABLE codes (
    code TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE config (
    genshin_uid      TEXT    NOT NULL PRIMARY KEY,
    auto_claim_codes INTEGER DEFAULT 1 NOT NULL
);

CREATE TABLE hoyo_cookie (
    cookie_id    INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    ltuid        TEXT    NOT NULL,
    ltoken       TEXT    NOT NULL,
    cookie_token TEXT    NOT NULL,
    account_id   TEXT    NOT NULL,
    lang         TEXT    NOT NULL
);
