SELECT c.id, c.category, c.slug, c.description,
  COALESCE ((
    SELECT sum(c2.crates_cnt)::int
    FROM categories as c2
    WHERE c2.slug = c.slug
    OR c2.slug LIKE c.slug || '::%'
  ), 0) as crates_cnt, c.created_at
FROM categories as c
WHERE c.category ILIKE $1 || '::%'
AND c.category NOT ILIKE $1 || '::%::%'
