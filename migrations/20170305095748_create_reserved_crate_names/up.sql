CREATE TABLE reserved_crate_names (
    name TEXT PRIMARY KEY
);
CREATE UNIQUE INDEX ON reserved_crate_names (canon_crate_name(name));
INSERT INTO reserved_crate_names (name) VALUES
    ('alloc'), ('arena'), ('ast'), ('builtins'), ('collections'),
    ('compiler-builtins'), ('compiler-rt'), ('compiletest'), ('core'), ('coretest'),
    ('debug'), ('driver'), ('flate'), ('fmt_macros'), ('grammar'), ('graphviz'),
    ('macro'), ('macros'), ('proc_macro'), ('rbml'), ('rust-installer'), ('rustbook'),
    ('rustc'), ('rustc_back'), ('rustc_borrowck'), ('rustc_driver'), ('rustc_llvm'),
    ('rustc_resolve'), ('rustc_trans'), ('rustc_typeck'), ('rustdoc'), ('rustllvm'),
    ('rustuv'), ('serialize'), ('std'), ('syntax'), ('test'), ('unicode');

CREATE FUNCTION ensure_crate_name_not_reserved() RETURNS trigger AS $$
BEGIN
    IF canon_crate_name(NEW.name) IN (
        SELECT canon_crate_name(name) FROM reserved_crate_names
    ) THEN
        RAISE EXCEPTION 'cannot upload crate with reserved name';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_ensure_crate_name_not_reserved
BEFORE INSERT OR UPDATE ON crates
FOR EACH ROW EXECUTE PROCEDURE ensure_crate_name_not_reserved();
