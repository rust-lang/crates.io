/**
 * Loads the README HTML for a crate version.
 *
 * @returns The README HTML string, or `null` if no README exists.
 * @throws Error If the request fails with a non-404/403 status.
 */
export async function loadReadme(
  fetch: typeof globalThis.fetch,
  crateName: string,
  versionNum: string,
): Promise<string | null> {
  let response = await fetch(`/api/v1/crates/${crateName}/${versionNum}/readme`);

  // 404/403 means no README (not an error)
  if (response.status === 404 || response.status === 403) {
    return null;
  }

  if (!response.ok) {
    throw new Error('Failed to load README');
  }

  return await response.text();
}
