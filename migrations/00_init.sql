create table if not exists users
(
    id          integer primary key autoincrement,
    name        text not null,
    password    text not null unique,
    domain      text not null
);
