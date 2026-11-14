import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { loadApi } from '#lib/utils/api.ts';

type Client = ReturnType<typeof createClient>;

export async function load({ fetch, parent }) {
  let { userPromise } = await parent();
  let user = await userPromise;

  if (!user) {
    error(401, 'This page requires authentication', { loginNeeded: true });
  }

  let client = createClient({ fetch });

  return { invites: await loadInvites(client, user.id) };
}

async function loadInvites(client: Client, userId: number) {
  let { crate_owner_invitations: invitations, users } = await loadApi(
    client.GET('/api/v1/crate_owner_invitations', { params: { query: { invitee_id: userId } } }),
    { message: 'Failed to load pending invites' },
  );

  let usersById = new Map(users.map(u => [u.id, u]));

  return invitations.map(inv => ({
    id: inv.id,
    crate_name: inv.crate_name,
    inviter_login: usersById.get(inv.inviter_id)?.login ?? 'unknown',
    created_at: inv.created_at,
  }));
}
