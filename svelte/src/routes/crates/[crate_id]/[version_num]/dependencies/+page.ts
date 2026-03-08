import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { loadCrateDescriptions } from '$lib/utils/crate-descriptions';

export async function load({ fetch, params }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let versionNum = params.version_num;

  let versionPromise = loadVersion(client, crateName, versionNum);
  let dependenciesPromise = loadDependencies(client, crateName, versionNum);

  let [version, dependencies] = await Promise.all([versionPromise, dependenciesPromise]);

  let descriptionMap = loadCrateDescriptions(
    client,
    dependencies.map(d => d.crate_id),
  );

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
