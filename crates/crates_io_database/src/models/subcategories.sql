SELECT c.id, c.category, c.slug, c.description,
  COALESCE ((
    SELECT count(distinct cc.crate_id)::int
    FROM categories as c2
    LEFT JOIN crates_categories cc ON cc.category_id = c2.id
    WHERE c2.slug = c.slug
    OR c2.slug LIKE c.slug || '::%'
  ), 0) as crates_cnt, c.created_at
FROM categories as c
WHERE c.category ILIKE $1 || '::%'
AND c.category NOT ILIKE $1 || '::%::%'
