drop trigger trigger_crates_set_updated_at
    on crates;

create trigger trigger_crates_set_updated_at
    before update
    on crates
    for each row
execute procedure set_updated_at_ignore_downloads();
