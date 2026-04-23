create table if not exists auth_accounts (
    id serial primary key,
    email varchar(255) not null unique,
    username varchar(50) not null unique,
    password_hash text not null
);
