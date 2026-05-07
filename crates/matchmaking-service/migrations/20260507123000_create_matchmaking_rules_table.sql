create table if not exists matchmaking_rules (
    id uuid primary key,
    ticket_key varchar(100) not null unique,
    required_players integer not null check (required_players >= 2),
    enabled boolean not null default true
);

insert into matchmaking_rules (id, ticket_key, required_players, enabled)
values ('11111111-1111-1111-1111-111111111111', 'duel', 2, true)
on conflict (ticket_key) do nothing;
