create table recordings (
    id serial primary key not null,
    user_id integer, -- maybe null
    uuid char(36) not null, -- UUIDv4; length of 32 with four hyphens
    rec_start timestamp not null,
    rec_end timestamp not null,
    status text not null,
    short_status varchar(32) not null,
    stage integer not null, -- stage enum
    channel varchar(32) not null
);
