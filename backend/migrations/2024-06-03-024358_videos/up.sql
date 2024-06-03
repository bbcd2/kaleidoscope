create table videos (
    id serial primary key not null,
    user_id integer, -- maybe null
    uuid char(36) not null, -- UUIDv4; length of 32 with four hyphens
    rec_start timestamp not null,
    rec_end timestamp not null,
    status varchar(128) not null,
    channel integer not null
);
