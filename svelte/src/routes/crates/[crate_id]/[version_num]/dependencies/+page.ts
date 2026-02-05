import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

const BULK_REQUEST_GROUP_SIZE = 10;

export async function load({ fetch, params }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let versionNum = params.version_num;

  let versionPromise = loadVersion(client, crateName, versionNum);
  let dependenciesPromise = loadDependencies(client, crateName, versionNum);

  let [version, dependencies] = await Promise.all([versionPromise, dependenciesPromise]);

  let descriptionMap = loadDescriptions(client, dependencies);

  return {
    requestedVersion: versionNum,
    version,
    dependencies,
    descriptionMap,
  };
}

async function loadVersion(client: ReturnType<typeof createClient>, name: string, version: string) {
  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}/{version}', { params: { path: { name, version } } });
  } catch (_error) {
    loadVersionError(name, version, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadVersionError(name, version, status);
  }

  return response.data.version;
}

function loadVersionError(name: string, version: string, status: number): never {
  if (status === 404) {
    error(404, { message: `${name}: Version ${version} not found` });
  } else {
    error(status, { message: `${name}: Failed to load version data`, tryAgain: true });
  }
}

async function loadDependencies(client: ReturnType<typeof createClient>, name: string, version: string) {
  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}/{version}/dependencies', {
      params: { path: { name, version } },
    });
  } catch (_error) {
    loadDependenciesError(name, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadDependenciesError(name, status);
  }

  return response.data.dependencies;
}

function loadDependenciesError(name: string, status: number): never {
  error(status, { message: `${name}: Failed to load dependencies`, tryAgain: true });
}

/**
 * Batch-loads crate descriptions for all dependencies.
 *
 * Collects unique crate IDs from the dependencies, splits them into batches
 * of 10, and fetches each batch via `GET /api/v1/crates?ids[]=...`. Returns
 * a map from crate ID to a promise that resolves to the description.
 */
function loadDescriptions(
  client: ReturnType<typeof createClient>,
  dependencies: { crate_id: string }[],
): Map<string, Promise<string | null>> {
  let uniqueIds = [...new Set(dependencies.map(d => d.crate_id))];

  let batches: string[][] = [];
  for (let i = 0; i < uniqueIds.length; i += BULK_REQUEST_GROUP_SIZE) {
    batches.push(uniqueIds.slice(i, i + BULK_REQUEST_GROUP_SIZE));
  }

  let descriptionMap = new Map<string, Promise<string | null>>();

  for (let batch of batches) {
    let batchPromise = loadCrateBatch(client, batch);

    for (let id of batch) {
      let promise = batchPromise.then(map => map.get(id) ?? null);
      descriptionMap.set(id, promise);
    }
  }

  return descriptionMap;
}

async function loadCrateBatch(
  client: ReturnType<typeof createClient>,
  ids: string[],
): Promise<Map<string, string | null>> {
  let response = await client.GET('/api/v1/crates', {
    params: { query: { 'ids[]': ids, per_page: ids.length } },
  });

  let map = new Map<string, string | null>();

  for (let crate of response.data?.crates ?? []) {
    map.set(crate.name, crate.description ?? null);
  }

  return map;
}
