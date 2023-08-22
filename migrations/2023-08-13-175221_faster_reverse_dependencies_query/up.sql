ALTER TABLE versions ADD COLUMN IF NOT EXISTS semver_no_prerelease public.semver_triple GENERATED ALWAYS AS (to_semver_no_prerelease(num)) STORED;
