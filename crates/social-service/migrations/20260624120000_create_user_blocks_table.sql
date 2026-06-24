create table if not exists user_blocks (
    id uuid primary key,
    blocker_id uuid not null references user_profiles(id) on delete cascade,
    blocked_id uuid not null references user_profiles(id) on delete cascade,
    created_at timestamptz not null default now(),
    check (blocker_id <> blocked_id)
);

create unique index if not exists user_blocks_pair_idx
on user_blocks (blocker_id, blocked_id);
