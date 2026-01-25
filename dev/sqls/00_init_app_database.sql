create table if not exists users
(
    id          integer primary key autoincrement,
    name        text not null,
    password    text not null unique,
    domain      text not null
);

insert into users (name, password, domain)
values ('project', '$argon2id$v=19$m=4096,t=3,p=1$aGFuc3BldGVy$sLdoPa9p5R6qqTqu1TVoAVSOxEBtocv1rzVaH1X4A7w','test')