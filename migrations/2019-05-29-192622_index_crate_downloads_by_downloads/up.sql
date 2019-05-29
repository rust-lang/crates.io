DROP MATERIALIZED VIEW recent_crate_downloads;
CREATE MATERIALIZED VIEW recent_crate_downloads (crate_id, downloads) AS
  SELECT crate_id, COALESCE(SUM(version_downloads.downloads), 0) FROM versions
    LEFT JOIN version_downloads
      ON version_downloads.version_id = versions.id
    WHERE version_downloads.date > date(CURRENT_TIMESTAMP - INTERVAL '90 days')
      OR version_downloads.date IS NULL
    GROUP BY crate_id;
CREATE UNIQUE INDEX recent_crate_downloads_crate_id ON recent_crate_downloads (crate_id);
CREATE INDEX ON recent_crate_downloads (downloads);
