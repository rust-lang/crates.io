CREATE OR REPLACE FUNCTION update_num_versions_from_versions() RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        INSERT INTO default_versions (crate_id, version_id, num_versions)
        VALUES (NEW.crate_id, NEW.id, 1)
        ON CONFLICT (crate_id) DO UPDATE
        SET num_versions = default_versions.num_versions + 1;
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        UPDATE default_versions
        SET num_versions = num_versions - 1
        WHERE crate_id = OLD.crate_id;
        RETURN OLD;
    END IF;
END
$$ LANGUAGE plpgsql;
