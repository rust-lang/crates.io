import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, parent }) {
  let { userPromise } = await parent();
  let user = await userPromise;

  if (!user) {
    error(401, { message: 'This page requires authentication', loginNeeded: true });
  }

  let client = createClient({ fetch });

  let [myCrates, myFollowing, stats, updates] = await Promise.all([
    loadCrates(client, { user_id: user.id }),
    loadCrates(client, { following: '1' }),
    loadStats(client, user.id),
    loadUpdates(client),
  ]);

  return {
    user,
    myCrates,
    myFollowing,
    totalDownloads: stats.total_downloads,
    updates,
  };
}

function loadError(status: number): never {
  error(status, { message: 'Failed to load dashboard data', tryAgain: true });
}

type Client = ReturnType<typeof createClient>;

async function loadCrates(client: Client, query: { user_id?: number; following?: string }) {
  let response;
  try {
    response = await client.GET('/api/v1/crates', { params: { query } });
  } catch (_error) {
    loadError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadError(status);
  }

  return response.data.crates;
}

async function loadStats(client: Client, userId: number) {
  let response;
  try {
    response = await client.GET('/api/v1/users/{id}/stats', { params: { path: { id: userId } } });
  } catch (_error) {
    loadError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadError(status);
  }

  return response.data;
}

async function loadUpdates(client: Client) {
  let response;
  try {
    response = await client.GET('/api/v1/me/updates');
  } catch (_error) {
    loadError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadError(status);
  }

  return response.data;
}
