import type { components } from '@crates-io/api-client';
import type { Version } from '../models/index.js';

import { serializeUser } from './user.js';

type ApiVersion = components['schemas']['Version'];

export function serializeVersion(version: Version): ApiVersion {
  return {
    id: version.id,
    crate: version.crate.name,
    num: version.num,
    dl_path: `/api/v1/crates/${version.crate.name}/${version.num}/download`,
    readme_path: `/api/v1/crates/${version.crate.name}/${version.num}/readme`,
    updated_at: version.updated_at,
    created_at: version.created_at,
    downloads: version.downloads,
    features: version.features,
    yanked: version.yanked,
    yank_message: version.yank_message,
    audit_actions: version.audit_actions.map(auditAction => ({
      action: auditAction.action,
      time: auditAction.time,
      user: serializeUser(auditAction.user),
    })),
    checksum: version.checksum,
    links: {
      authors: `/api/v1/crates/${version.crate.name}/${version.num}/authors`,
      dependencies: `/api/v1/crates/${version.crate.name}/${version.num}/dependencies`,
      version_downloads: `/api/v1/crates/${version.crate.name}/${version.num}/downloads`,
    },
    crate_size: version.crate_size,
    bin_names: null,
    description: null,
    documentation: null,
    edition: null,
    has_lib: null,
    homepage: null,
    lib_links: null,
    license: version.license,
    repository: null,
    rust_version: version.rust_version,
    trustpub_data: version.trustpub_data,
    linecounts: version.linecounts,
    published_by: version.publishedBy ? serializeUser(version.publishedBy) : null,
  };
}
