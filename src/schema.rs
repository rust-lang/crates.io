// This file can be regenerated with `diesel print-schema`

table! {
    badges (crate_id,
    badge_type) {
        crate_id -> Int4,
        badge_type -> Varchar,
        attributes -> Jsonb,
    }
}

table! {
    categories (id) {
        id -> Int4,
        category -> Varchar,
        slug -> Varchar,
        description -> Varchar,
        crates_cnt -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    crate_downloads (crate_id, date) {
        crate_id -> Int4,
        downloads -> Int4,
        date -> Date,
    }
}

table! {
    crate_owners (crate_id, owner_id, owner_kind) {
        crate_id -> Int4,
        owner_id -> Int4,
        created_at -> Timestamp,
        created_by -> Nullable<Int4>,
        deleted -> Bool,
        updated_at -> Timestamp,
        owner_kind -> Int4,
    }
}

table! {
    crates (id) {
        id -> Int4,
        name -> Varchar,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        downloads -> Int4,
        description -> Nullable<Varchar>,
        homepage -> Nullable<Varchar>,
        documentation -> Nullable<Varchar>,
        readme -> Nullable<Varchar>,
        textsearchable_index_col -> ::diesel_full_text_search::TsVector,
        license -> Nullable<Varchar>,
        repository -> Nullable<Varchar>,
        max_upload_size -> Nullable<Int4>,
    }
}

table! {
    crates_categories (crate_id,
    category_id) {
        crate_id -> Int4,
        category_id -> Int4,
    }
}

table! {
    crates_keywords (crate_id,
    keyword_id) {
        crate_id -> Int4,
        keyword_id -> Int4,
    }
}

table! {
    dependencies (id) {
        id -> Int4,
        version_id -> Int4,
        crate_id -> Int4,
        req -> Varchar,
        optional -> Bool,
        default_features -> Bool,
        features -> Array<Text>,
        target -> Nullable<Varchar>,
        kind -> Int4,
    }
}

table! {
    follows (user_id,
    crate_id) {
        user_id -> Int4,
        crate_id -> Int4,
    }
}

table! {
    keywords (id) {
        id -> Int4,
        keyword -> Varchar,
        crates_cnt -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    metadata (total_downloads) {
        total_downloads -> Int8,
    }
}

table! {
    reserved_crate_names (name) {
        name -> Text,
    }
}

table! {
    teams (id) {
        id -> Int4,
        login -> Varchar,
        github_id -> Int4,
        name -> Nullable<Varchar>,
        avatar -> Nullable<Varchar>,
    }
}

table! {
    users (id) {
        id -> Int4,
        email -> Nullable<Varchar>,
        gh_access_token -> Varchar,
        api_token -> Varchar,
        gh_login -> Varchar,
        name -> Nullable<Varchar>,
        gh_avatar -> Nullable<Varchar>,
        gh_id -> Int4,
    }
}

table! {
    version_authors (id) {
        id -> Int4,
        version_id -> Int4,
        user_id -> Nullable<Int4>,
        name -> Varchar,
    }
}

table! {
    version_downloads (id) {
        id -> Int4,
        version_id -> Int4,
        downloads -> Int4,
        counted -> Int4,
        date -> Date,
        processed -> Bool,
    }
}

table! {
    versions (id) {
        id -> Int4,
        crate_id -> Int4,
        num -> Varchar,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        downloads -> Int4,
        features -> Nullable<Varchar>,
        yanked -> Bool,
    }
}
