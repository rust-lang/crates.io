SELECT c.id, c.category, c.slug, c.description,
  COALESCE((
    SELECT sum(c2.crates_cnt)::int from categories c2
    WHERE path <@ subltree(c.path, 0, 2)
  ), 0) as crates_cnt, c.created_at
FROM categories c
WHERE c.path @> (select path from categories where slug = $1)
AND c.slug <> $1
ORDER BY c.path
