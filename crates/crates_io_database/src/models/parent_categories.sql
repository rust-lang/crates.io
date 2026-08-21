SELECT c.id, c.category, c.slug, c.description,
  COALESCE((
    SELECT count(distinct cc.crate_id)::int from categories c2
    LEFT JOIN crates_categories cc ON cc.category_id = c2.id
    WHERE c2.path <@ subltree(c.path, 0, 2)
  ), 0) as crates_cnt, c.created_at
FROM categories c
WHERE c.path @> (select path from categories where slug = $1)
AND c.slug <> $1
ORDER BY c.path
