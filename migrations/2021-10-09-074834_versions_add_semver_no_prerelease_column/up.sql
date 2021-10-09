-- add the new column.

ALTER TABLE versions
    ADD COLUMN semver_no_prerelease semver_triple;

-- fill the new column with data incrementally to avoid table locking for
-- extended time periods.

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-01-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-02-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-03-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-04-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-05-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-06-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-07-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-08-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-09-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-10-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-11-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2016-12-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-01-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-02-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-03-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-04-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-05-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-06-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-07-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-08-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-09-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-10-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-11-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2017-12-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-01-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-02-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-03-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-04-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-05-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-06-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-07-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-08-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-09-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-10-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-11-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2018-12-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-01-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-02-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-03-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-04-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-05-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-06-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-07-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-08-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-09-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-10-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-11-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2019-12-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-01-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-02-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-03-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-04-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-05-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-06-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-07-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-08-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-09-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-10-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-11-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2020-12-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-01-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-02-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-03-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-04-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-05-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-06-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-07-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-08-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-09-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-10-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-11-01';

UPDATE versions
SET semver_no_prerelease = to_semver_no_prerelease(num)
WHERE semver_no_prerelease IS NULL
  AND created_at < '2021-12-01';
