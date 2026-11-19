import { createClient } from '@crates-io/api-client';

import { loadSiteMetadata } from './site-metadata';

const FALLBACK_CDN_BASE = 'https://static.crates.io';

/**
 * Resolves the base URL that crate files are served from.
 */
export function cdnBase(metadata: { cdn_base?: string }): string {
  return metadata?.cdn_base ?? FALLBACK_CDN_BASE;
}

/**
 * Loads the site metadata and resolves the base URL that crate files are served from.
 *
 * @throws Error If the site metadata cannot be loaded.
 */
export async function loadCdnBase(fetch: typeof globalThis.fetch): Promise<string> {
  let { data } = await loadSiteMetadata(createClient({ fetch }));
  if (!data) {
    throw new Error('Failed to load site metadata');
  }

  return cdnBase(data);
}
