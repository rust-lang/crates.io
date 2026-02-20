import { error } from '@sveltejs/kit';

export async function load({ parent }) {
  let { userPromise } = await parent();
  let user = await userPromise;

  if (!user) {
    error(401, { message: 'This page requires authentication', loginNeeded: true });
  }
}
