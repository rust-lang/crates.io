-- Rewrite `random_string` to draw randomness from `pgcrypto`'s
-- `gen_random_bytes()` instead of the built-in `random()` function, which the
-- PostgreSQL documentation explicitly warns is not cryptographically secure.
--
-- A naive `get_byte(...) % 62` would introduce modulo bias (256 is not a
-- multiple of 62, so the values 0..7 would appear slightly more often). To get
-- a uniform distribution over the 62-character alphabet we use rejection
-- sampling: 248 = 4 * 62 is the largest multiple of 62 that fits in a byte, so
-- we discard any byte >= 248 and try again. The expected number of bytes
-- consumed per output character is 256/248 ~= 1.03.
CREATE OR REPLACE FUNCTION random_string(int4) RETURNS text AS $$
DECLARE
  alphabet CONSTANT text := 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  result text := '';
  b int4;
BEGIN
  WHILE length(result) < $1 LOOP
    b := get_byte(gen_random_bytes(1), 0);
    CONTINUE WHEN b >= 248;
    result := result || substr(alphabet, (b % 62) + 1, 1);
  END LOOP;
  RETURN result;
END;
$$ LANGUAGE plpgsql VOLATILE;

COMMENT ON FUNCTION random_string(int4) IS
  'Returns a cryptographically random alphanumeric string of the requested length, drawing from pgcrypto''s gen_random_bytes() with rejection sampling for a uniform distribution.';
