DROP MATERIALIZED VIEW recent_crate_downloads;
CREATE MATERIALIZED VIEW recent_crate_downloads (crate_id, downloads, monthly, weekly) AS
  SELECT
    crate_id,
    SUM(version_downloads.downloads),
    SUM(version_downloads.downloads) FILTER (WHERE version_downloads.date > CURRENT_DATE - 30),
    SUM(version_downloads.downloads) FILTER (WHERE version_downloads.date > CURRENT_DATE - 7)
  FROM version_downloads
    INNER JOIN versions
      ON version_downloads.version_id = versions.id
    WHERE version_downloads.date > CURRENT_DATE - 90
    GROUP BY crate_id;
CREATE UNIQUE INDEX recent_crate_downloads_crate_id ON recent_crate_downloads (crate_id);
