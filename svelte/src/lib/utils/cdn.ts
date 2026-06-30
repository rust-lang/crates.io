const FALLBACK_CDN_BASE = 'https://static.crates.io';

/**
 * Resolves the base URL that crate files are served from.
 */
export function cdnBase(metadata: { cdn_base?: string }): string {
  return metadata?.cdn_base ?? FALLBACK_CDN_BASE;
}
