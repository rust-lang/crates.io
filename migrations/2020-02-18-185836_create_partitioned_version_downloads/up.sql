CREATE TABLE version_downloads_part (
  version_id INTEGER NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
  downloads INTEGER NOT NULL DEFAULT 1,
  counted INTEGER NOT NULL DEFAULT 0,
  date DATE NOT NULL DEFAULT CURRENT_DATE,
  PRIMARY KEY (version_id, date)
) PARTITION BY RANGE (date);

CREATE TABLE version_downloads_default PARTITION OF version_downloads_part DEFAULT;

COMMENT ON TABLE version_downloads_default IS
  'This table should always be empty. We partition by quarter (or perhaps
  more frequently in the future), and we create the partitions a year in
  advance. If data ends up here, something has gone wrong with partition
  creation. This table exists so we don''t lose data if that happens, and
  so we have a way to detect this happening programatically.';

CREATE TABLE version_downloads_pre_2017 PARTITION OF version_downloads_part
  FOR VALUES FROM (MINVALUE) TO ('2017-01-01');

CREATE TABLE version_downloads_2017 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2017-01-01') TO ('2018-01-01');

CREATE TABLE version_downloads_2018_q1 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2018-01-01') TO ('2018-04-01');

CREATE TABLE version_downloads_2018_q2 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2018-04-01') TO ('2018-07-01');

CREATE TABLE version_downloads_2018_q3 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2018-07-01') TO ('2018-10-01');

CREATE TABLE version_downloads_2018_q4 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2018-10-01') TO ('2019-01-01');

CREATE TABLE version_downloads_2019_q1 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2019-01-01') TO ('2019-04-01');

CREATE TABLE version_downloads_2019_q2 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2019-04-01') TO ('2019-07-01');

CREATE TABLE version_downloads_2019_q3 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2019-07-01') TO ('2019-10-01');

CREATE TABLE version_downloads_2019_q4 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2019-10-01') TO ('2020-01-01');

CREATE TABLE version_downloads_2020_q1 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2020-01-01') TO ('2020-04-01');

CREATE TABLE version_downloads_2020_q2 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2020-04-01') TO ('2020-07-01');

CREATE TABLE version_downloads_2020_q3 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2020-07-01') TO ('2020-10-01');

CREATE TABLE version_downloads_2020_q4 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2020-10-01') TO ('2021-01-01');

CREATE TABLE version_downloads_2021_q1 PARTITION OF version_downloads_part
  FOR VALUES FROM ('2021-01-01') TO ('2021-04-01');

CREATE FUNCTION update_partitioned_version_downloads() RETURNS TRIGGER AS $$
BEGIN
  IF NEW IS DISTINCT FROM OLD THEN
    INSERT INTO version_downloads_part (version_id, downloads, counted, date)
    VALUES (NEW.version_id, NEW.downloads, NEW.counted, NEW.date)
    ON CONFLICT (version_id, date) DO UPDATE
    SET downloads = EXCLUDED.downloads, counted = EXCLUDED.counted;
  END IF;
  RETURN NULL;
END;
$$ LANGUAGE PLpgSQL;

CREATE TRIGGER update_partitioned_version_downloads_trigger
  AFTER INSERT OR UPDATE ON version_downloads
  FOR EACH ROW EXECUTE FUNCTION update_partitioned_version_downloads();
