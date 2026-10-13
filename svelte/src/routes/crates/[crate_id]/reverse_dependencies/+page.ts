import type { components } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';
import { loadCrateDescriptions } from '#lib/utils/crate-descriptions.ts';

const PER_PAGE = 10;

type Version = components['schemas']['Version'];

export async function load({ fetch, params, url }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);

  let { dependencies, versions, total } = await loadReverseDependencies(client, crateName, page);

  let versionMap = new Map<number, Version>(versions.map(v => [v.id, v]));

  let enrichedDeps = dependencies.map(dep => {
    let version = versionMap.get(dep.version_id);
    return {
      ...dep,
      dependentCrateName: version?.crate ?? dep.crate_id,
    };
  });

  let descriptionMap = loadCrateDescriptions(
    client,
    enrichedDeps.map(d => d.dependentCrateName),
  );

  return {
    dependencies: enrichedDeps,
    total,
    page,
    perPage: PER_PAGE,
    descriptionMap,
  };
}

async function loadReverseDependencies(client: ReturnType<typeof createClient>, name: string, page: number) {
  let { dependencies, versions, meta } = await loadApi(
    client.GET('/api/v1/crates/{name}/reverse_dependencies', {
      params: { path: { name }, query: { page, per_page: PER_PAGE } },
    }),
    { message: `${name}: Failed to load dependents` },
  );

  return { dependencies, versions, total: meta.total };
}
