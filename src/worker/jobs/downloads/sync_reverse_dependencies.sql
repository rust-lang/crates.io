UPDATE reverse_dependencies
SET dependent_downloads = crate_downloads.downloads
FROM crate_downloads
WHERE reverse_dependencies.dependent_crate_id = crate_downloads.crate_id
  AND reverse_dependencies.dependent_downloads IS DISTINCT FROM crate_downloads.downloads
