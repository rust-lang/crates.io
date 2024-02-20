create table if not exists badges
(
    crate_id   integer not null
        constraint fk_badges_crate_id
            references crates
            on delete cascade,
    badge_type varchar not null,
    attributes jsonb   not null,
    primary key (crate_id, badge_type)
);
