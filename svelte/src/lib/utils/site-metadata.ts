import { createClient } from '@crates-io/api-client';

type ApiClient = ReturnType<typeof createClient>;

// Cached at module scope, so the request fires once and is shared by every
// caller. A failed request clears the cache so a later call retries.
let request: ReturnType<typeof fetchSiteMetadata> | undefined;

export function loadSiteMetadata(client: ApiClient) {
  return (request ??= fetchSiteMetadata(client));
}

async function fetchSiteMetadata(client: ApiClient) {
  try {
    let result = await client.GET('/api/v1/site_metadata');
    if (!result.data) {
      request = undefined;
    }
    return result;
  } catch (error) {
    request = undefined;
    throw error;
  }
}

/** Clears the cached request so each test starts from a clean slate. */
export function resetSiteMetadataCache() {
  request = undefined;
}
