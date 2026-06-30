import { createClient } from '@crates-io/api-client';

import { cdnBase } from './cdn';
import { loadSiteMetadata } from './site-metadata';

/**
 * Loads the README HTML for a crate version.
 *
 * @returns The README HTML string, or `null` if no README exists.
 * @throws Error If site metadata cannot be loaded, or the README request fails
 *   with a non-404/403 status.
 */
export async function loadReadme(
  fetch: typeof globalThis.fetch,
  crateName: string,
  versionNum: string,
): Promise<string | null> {
  let { data } = await loadSiteMetadata(createClient({ fetch }));
  if (!data) {
    throw new Error('Failed to load README');
  }

  let base = cdnBase(data);

  // `encodeURIComponent` matches the backend's `+` -> `%2B` encoding for versions with build metadata.
  let version = encodeURIComponent(versionNum);
  let response = await fetch(`${base}/readmes/${crateName}/${crateName}-${version}.html`);

  // 404/403 means no README (not an error)
  if (response.status === 404 || response.status === 403) {
    return null;
  }

  if (!response.ok) {
    throw new Error('Failed to load README');
  }

  return await response.text();
}
