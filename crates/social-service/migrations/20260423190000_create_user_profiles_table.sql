create table if not exists user_profiles (
    id uuid primary key,
    first_name varchar(100) not null,
    last_name varchar(100) not null
);
