DROP MATERIALIZED VIEW recent_crate_downloads;
CREATE MATERIALIZED VIEW recent_crate_downloads (crate_id, downloads) AS
  SELECT crate_id, SUM(version_downloads.downloads) FROM version_downloads
    INNER JOIN versions
      ON version_downloads.version_id = versions.id
    WHERE version_downloads.date > date(CURRENT_TIMESTAMP - INTERVAL '90 days')
    GROUP BY crate_id;
CREATE UNIQUE INDEX recent_crate_downloads_crate_id ON recent_crate_downloads (crate_id);
