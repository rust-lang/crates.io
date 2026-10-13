import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, parent }) {
  let { crate, ownersPromise } = await parent();

  let client = createClient({ fetch });
  let crateName = crate.name;

  let [owners, githubConfigs, gitlabConfigs] = await Promise.all([
    ownersPromise,
    loadGitHubConfigs(client, crateName),
    loadGitLabConfigs(client, crateName),
  ]);

  return { owners, githubConfigs, gitlabConfigs };
}

async function loadGitHubConfigs(client: ReturnType<typeof createClient>, crateName: string) {
  let { github_configs } = await loadApi(
    client.GET('/api/v1/trusted_publishing/github_configs', { params: { query: { crate: crateName } } }),
    { message: 'Failed to load crate data' },
  );

  return github_configs;
}

async function loadGitLabConfigs(client: ReturnType<typeof createClient>, crateName: string) {
  let { gitlab_configs } = await loadApi(
    client.GET('/api/v1/trusted_publishing/gitlab_configs', { params: { query: { crate: crateName } } }),
    { message: 'Failed to load crate data' },
  );

  return gitlab_configs;
}
