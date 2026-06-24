create table if not exists friend_requests (
    id uuid primary key,
    requester_id uuid not null references user_profiles(id) on delete cascade,
    recipient_id uuid not null references user_profiles(id) on delete cascade,
    status varchar(20) not null check (status in ('pending', 'declined', 'accepted')),
    created_at timestamptz not null default now(),
    responded_at timestamptz,
    check (requester_id <> recipient_id)
);

create unique index if not exists friend_requests_pending_pair_idx
on friend_requests (
    least(requester_id, recipient_id),
    greatest(requester_id, recipient_id)
)
where status = 'pending';
