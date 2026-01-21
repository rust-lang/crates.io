/**
 * Generates a Package URL (PURL) for a crate version.
 *
 * For the main crates.io registry, returns a bare PURL like `pkg:cargo/serde@1.0.0`.
 * For other hosts (staging, custom registries), appends a `repository_url` parameter.
 *
 * @see https://github.com/package-url/purl-spec
 */
export function getPurl(crateName: string, version: string): string {
  let basePurl = `pkg:cargo/${crateName}@${version}`;
  let host = globalThis.location.host;

  if (host === 'crates.io' || import.meta.env.STORYBOOK) {
    return basePurl;
  }

  let repositoryUrl = `https://${host}/`;
  return `${basePurl}?repository_url=${encodeURIComponent(repositoryUrl)}`;
}
