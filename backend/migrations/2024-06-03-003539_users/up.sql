create table users (
    id serial primary key not null,
    username varchar(32) not null,
    max_length_seconds integer not null default 3600, -- 1 hour
    can_upload boolean not null default true,
    can_delete boolean not null default false,
    superuser boolean not null default false
);