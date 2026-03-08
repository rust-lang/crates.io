import { createClient } from '@crates-io/api-client';

const BATCH_SIZE = 10;

/**
 * Batch-loads crate descriptions for a list of crate names.
 *
 * Splits the names into batches of 10 and fetches each batch via
 * `GET /api/v1/crates?ids[]=...`. Returns a map from crate name
 * to a promise that resolves to the description.
 */
export function loadCrateDescriptions(
  client: ReturnType<typeof createClient>,
  crateNames: string[],
): Map<string, Promise<string | null>> {
  let uniqueNames = [...new Set(crateNames)];

  let batches: string[][] = [];
  for (let i = 0; i < uniqueNames.length; i += BATCH_SIZE) {
    batches.push(uniqueNames.slice(i, i + BATCH_SIZE));
  }

  let descriptionMap = new Map<string, Promise<string | null>>();

  for (let batch of batches) {
    let batchPromise = loadBatch(client, batch);

    for (let name of batch) {
      let promise = batchPromise.then(map => map.get(name) ?? null);
      descriptionMap.set(name, promise);
    }
  }

  return descriptionMap;
}

async function loadBatch(client: ReturnType<typeof createClient>, ids: string[]): Promise<Map<string, string | null>> {
  let response = await client.GET('/api/v1/crates', {
    params: { query: { 'ids[]': ids, per_page: ids.length } },
  });

  let map = new Map<string, string | null>();

  for (let crate of response.data?.crates ?? []) {
    map.set(crate.name, crate.description ?? null);
  }

  return map;
}
