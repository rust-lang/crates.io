DELETE FROM badges WHERE crate_id NOT IN (SELECT id FROM crates);
ALTER TABLE badges
    ADD CONSTRAINT fk_badges_crate_id FOREIGN KEY (crate_id) REFERENCES crates(id) ON DELETE CASCADE;
