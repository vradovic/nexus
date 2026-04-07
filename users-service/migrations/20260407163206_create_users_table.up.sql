create table users (
	id serial primary key,
	username varchar(50) not null unique,
	email varchar(100) not null unique
);
