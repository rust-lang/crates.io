CREATE FUNCTION random_string(int4) RETURNS text AS $$
  SELECT (array_to_string(array(
    SELECT substr(
      'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789',
      floor(random() * 62)::int4 + 1,
      1
    ) FROM generate_series(1, $1)
  ), ''))
$$ LANGUAGE SQL;

ALTER TABLE crate_owner_invitations
ALTER COLUMN token
SET DEFAULT random_string (26);

ALTER TABLE emails
ALTER COLUMN token
SET DEFAULT random_string (26);
