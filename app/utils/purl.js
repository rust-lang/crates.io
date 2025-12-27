import window from 'ember-window-mock';

/**
 * Adds a repository_url query parameter to a PURL string based on the host.
 *
 * @param {string} purl - The base PURL string (e.g., "pkg:cargo/serde@1.0.0")
 * @param {string} [host] - The host to use for repository URL. Defaults to current window location host.
 * @returns {string} The PURL with repository_url parameter added, or unchanged if host is crates.io
 */
export function addRegistryUrl(purl) {
  let host = window.location.host;

  // Don't add repository_url for the main crates.io registry
  if (host === 'crates.io') {
    return purl;
  }

  // Add repository_url query parameter
  let repositoryUrl = `https://${host}/`;
  let separator = purl.includes('?') ? '&' : '?';
  return `${purl}${separator}repository_url=${encodeURIComponent(repositoryUrl)}`;
}
