create table if not exists friendships (
    id uuid primary key,
    user_a_id uuid not null references user_profiles(id) on delete cascade,
    user_b_id uuid not null references user_profiles(id) on delete cascade,
    created_at timestamptz not null default now(),
    check (user_a_id <> user_b_id)
);

create unique index if not exists friendships_pair_idx
on friendships (
    least(user_a_id, user_b_id),
    greatest(user_a_id, user_b_id)
);
