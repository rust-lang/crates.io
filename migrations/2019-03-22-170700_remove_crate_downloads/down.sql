CREATE TABLE crate_downloads (
  crate_id        INTEGER NOT NULL REFERENCES crates (id),
  downloads       INTEGER NOT NULL,
  date            DATE NOT NULL,
  PRIMARY KEY (crate_id, date)
);
CREATE INDEX "index_crate_downloads_crate_id" ON crate_downloads (crate_id);
CREATE INDEX "index_crate_downloads_date" ON crate_downloads (date);

INSERT INTO crate_downloads (crate_id, downloads, date)
  SELECT crate_id, sum(version_downloads.downloads), date
  FROM version_downloads
  INNER JOIN versions
    ON version_downloads.version_id = versions.id
  GROUP BY (crate_id, date);
