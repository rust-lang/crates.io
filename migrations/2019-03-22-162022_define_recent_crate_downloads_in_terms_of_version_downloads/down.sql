DROP MATERIALIZED VIEW recent_crate_downloads;
CREATE MATERIALIZED VIEW recent_crate_downloads (crate_id, downloads) AS
  SELECT crate_id, SUM(downloads) FROM crate_downloads
    WHERE date > date(CURRENT_TIMESTAMP - INTERVAL '90 days')
    GROUP BY crate_id;
CREATE UNIQUE INDEX recent_crate_downloads_crate_id ON recent_crate_downloads (crate_id);
