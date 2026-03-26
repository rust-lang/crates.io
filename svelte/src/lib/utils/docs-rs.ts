export interface DocsRsStatus {
  doc_status?: boolean;
}

/**
 * Fetches the docs.rs build status for a crate version.
 *
 * Returns `null` if the request fails (e.g. 404, 500, network error).
 */
export async function loadDocsRsStatus(
  fetch: typeof globalThis.fetch,
  crateName: string,
  versionNum: string,
): Promise<DocsRsStatus | null> {
  try {
    let response = await fetch(`https://docs.rs/crate/${crateName}/${versionNum}/status.json`);
    if (!response.ok) {
      return null;
    }

    return await response.json();
  } catch {
    return null;
  }
}
