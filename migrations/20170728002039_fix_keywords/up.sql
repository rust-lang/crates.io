-- For all keywords that have a corresponding lowercased keyword,
-- (these keywords should not have been created but there was a bug
-- that created them; that has been fixed and no more are being created)
WITH messed_up_keywords AS (
    select keywords.id as upper_id, k.id as lower_id
    from keywords
    inner join keywords as k on LOWER(keywords.keyword) = k.keyword
    where LOWER(keywords.keyword) != keywords.keyword
)
-- Find all the crates that use the uppercased keyword BUT NOT the lowercased keyword
-- (many crates are associated with both lower and upper cased because of a bug)
, messed_up_crates AS (
    select crate_id, upper_id, lower_id
    from crates_keywords
    inner join messed_up_keywords on crates_keywords.keyword_id = messed_up_keywords.upper_id
    where messed_up_keywords.lower_id not in (
        select keyword_id
        from crates_keywords as ck
        where ck.crate_id = crates_keywords.crate_id
    )
)
-- Associate these crates with the lowercased keyword AS WELL AS the uppercased keyword
INSERT INTO crates_keywords (crate_id, keyword_id)
SELECT crate_id, lower_id as keyword_id
FROM messed_up_crates
;

-- For all keywords that have a corresponding lowercased keyword,
-- (this is repeated exactly from above)
WITH messed_up_keywords AS (
    select keywords.id as upper_id, k.id as lower_id
    from keywords
    inner join keywords as k on LOWER(keywords.keyword) = k.keyword
    where LOWER(keywords.keyword) != keywords.keyword
)
-- Delete records associating crates to the uppercased keyword
DELETE
FROM crates_keywords
WHERE crates_keywords.keyword_id IN (
    SELECT upper_id FROM messed_up_keywords
)
;

-- For all keywords that have a corresponding lowercased keyword,
-- (this is repeated exactly from above)
WITH messed_up_keywords AS (
    select keywords.id as upper_id, k.id as lower_id
    from keywords
    inner join keywords as k on LOWER(keywords.keyword) = k.keyword
    where LOWER(keywords.keyword) != keywords.keyword
)
-- Delete the uppercased keyword
-- No more crates should be associated with these keywords because of
-- the previous delete.
DELETE
FROM keywords
WHERE keywords.id IN (
    SELECT upper_id FROM messed_up_keywords
)
;

-- For all keywords that are not properly lowercased but do not
-- have a corresponding lowercased keyword, update them to be
-- lower cased, preserving any crate associations using them.
UPDATE keywords
SET keyword = lower(keyword)
WHERE keyword != lower(keyword)
;
