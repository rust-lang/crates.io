DROP TRIGGER trigger_crates_tsvector_update ON crates;

CREATE TRIGGER trigger_crates_tsvector_update BEFORE INSERT OR UPDATE OF updated_at
ON crates
FOR EACH ROW EXECUTE PROCEDURE trigger_crates_name_search();
