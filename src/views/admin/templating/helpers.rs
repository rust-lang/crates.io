use cargo_registry_index::Repository;

pub(super) fn crate_index_path(name: &str) -> String {
    String::from(
        Repository::relative_index_file(name)
            .to_str()
            .expect("invalid UTF-8 in crate name"),
    )
}
