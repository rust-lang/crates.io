import type { ResolvedPathname, RouteId } from '$app/types';

import { resolve } from '$app/paths';

/**
 * Builds the version switcher's link to `version_num`. It keeps the current
 * subpage, including the file path in the code browser, and links to the
 * version's main page from anywhere else.
 */
export function versionHref(
  routeId: RouteId | null,
  params: { path?: string },
  crate_id: string,
  version_num: string,
): ResolvedPathname {
  switch (routeId) {
    case '/crates/[crate_id]/[version_num]/dependencies': {
      return resolve('/crates/[crate_id]/[version_num]/dependencies', { crate_id, version_num });
    }
    case '/crates/[crate_id]/[version_num]/code/[...path]': {
      let path = params.path ?? '';
      return resolve('/crates/[crate_id]/[version_num]/code/[...path]', { crate_id, version_num, path });
    }
    default: {
      return resolve('/crates/[crate_id]/[version_num]', { crate_id, version_num });
    }
  }
}
