import { error } from '@sveltejs/kit';

export async function load({ parent }) {
  let { userPromise, ownersPromise } = await parent();

  let user = await userPromise;
  if (!user) {
    error(401, { message: 'This page requires authentication', loginNeeded: true });
  }

  let owners = await ownersPromise;
  let isOwner = owners.some(o => o.kind === 'user' && o.id === user.id);
  if (!isOwner) {
    error(403, { message: 'This page is only accessible by crate owners' });
  }
}
