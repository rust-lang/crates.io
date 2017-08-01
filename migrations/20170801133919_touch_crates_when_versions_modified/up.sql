CREATE OR REPLACE FUNCTION touch_crate_on_version_modified() RETURNS trigger AS $$
BEGIN
  IF (
    TG_OP = 'INSERT' OR
    NEW.updated_at IS DISTINCT FROM OLD.updated_at
  ) THEN
    UPDATE crates SET updated_at = CURRENT_TIMESTAMP WHERE
      crates.id = NEW.crate_id;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER touch_crate BEFORE INSERT OR UPDATE ON versions
  FOR EACH ROW EXECUTE PROCEDURE touch_crate_on_version_modified();
