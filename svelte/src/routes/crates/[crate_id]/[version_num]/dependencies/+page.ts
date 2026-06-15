import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { loadNativeReplacements } from '$lib/data/native-replacements';
import { loadCrateDescriptions } from '$lib/utils/crate-descriptions';

export async function load({ fetch, params }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let versionNum = params.version_num;

  let [dependencies, nativeReplacements] = await Promise.all([
    loadDependencies(client, crateName, versionNum),
    loadNativeReplacements(fetch),
  ]);

  let descriptionMap = loadCrateDescriptions(
    client,
    dependencies.map(d => d.crate_id),
  );

  return { dependencies, descriptionMap, nativeReplacements };
}

async function loadDependencies(client: ReturnType<typeof createClient>, name: string, version: string) {
  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}/{version}/dependencies', {
      params: { path: { name, version } },
    });
  } catch {
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
