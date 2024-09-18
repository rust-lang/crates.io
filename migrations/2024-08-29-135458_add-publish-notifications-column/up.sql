alter table users
    add column publish_notifications boolean not null default true;

comment on column users.publish_notifications is 'Whether or not the user wants to receive notifications when a package they own is published';
