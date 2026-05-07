alter table auth_accounts
add column if not exists role varchar(20) not null default 'player';
