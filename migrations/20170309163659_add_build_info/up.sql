CREATE TABLE build_info (
  version_id       INTEGER NOT NULL,
  rust_version     VARCHAR NOT NULL,
  target           VARCHAR NOT NULL,
  passed           BOOLEAN NOT NULL,
  created_at       TIMESTAMP NOT NULL DEFAULT now(),
  updated_at       TIMESTAMP NOT NULL DEFAULT now(),
  PRIMARY KEY (version_id, rust_version, target)
);
SELECT diesel_manage_updated_at('build_info');
