SELECT
  c.id,
  c.category,
  c.slug,
  c.description,
  sum(c2.crates_cnt)::int as crates_cnt,
  c.created_at
FROM categories as c
INNER JOIN categories c2 ON split_part(c2.slug, '::', 1) = c.slug
WHERE split_part(c.slug, '::', 1) = c.slug
GROUP BY c.id
{} LIMIT $1 OFFSET $2
