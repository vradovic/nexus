create table if not exists chat_messages (
    id uuid primary key,
    channel varchar(255) not null,
    sender_id uuid not null,
    body text not null,
    created_at timestamptz not null default now()
);

create index if not exists chat_messages_channel_created_at_idx
on chat_messages (channel, created_at desc);
