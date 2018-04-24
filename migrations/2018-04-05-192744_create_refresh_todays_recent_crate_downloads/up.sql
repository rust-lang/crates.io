CREATE FUNCTION refresh_todays_recent_crate_downloads() RETURNS void AS $$
BEGIN
  DROP INDEX IF EXISTS todays_recent_crate_downloads;
  EXECUTE 'CREATE INDEX todays_recent_crate_downloads
   ON crate_downloads (date)
   WHERE date > date(''' || CURRENT_DATE::text || '''::date - INTERVAL ''90 days'')';
  ANALYZE crate_downloads;
END;
$$ LANGUAGE plpgsql;
