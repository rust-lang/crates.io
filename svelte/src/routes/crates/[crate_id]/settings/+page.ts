import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { isRetryableStatus } from '#lib/utils/api.ts';

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

function loadError(status: number, details?: string): never {
  error(status, 'Failed to load crate data', { details, tryAgain: isRetryableStatus(status) });
}

async function loadGitHubConfigs(client: ReturnType<typeof createClient>, crateName: string) {
  let response;
  try {
    response = await client.GET('/api/v1/trusted_publishing/github_configs', {
      params: { query: { crate: crateName } },
    });
  } catch {
    loadError(504);
  }

  let status = response.response.status;
  if (response.error) {
    let details = response.error?.errors?.[0]?.detail;
    loadError(status, details);
  }

  return response.data.github_configs;
}

async function loadGitLabConfigs(client: ReturnType<typeof createClient>, crateName: string) {
  let response;
  try {
    response = await client.GET('/api/v1/trusted_publishing/gitlab_configs', {
      params: { query: { crate: crateName } },
    });
  } catch {
    loadError(504);
  }

  let status = response.response.status;
  if (response.error) {
    let details = response.error?.errors?.[0]?.detail;
    loadError(status, details);
  }

  return response.data.gitlab_configs;
}
