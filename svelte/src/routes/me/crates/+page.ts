import { resolve } from '$app/paths';
import { error, redirect } from '@sveltejs/kit';

export async function load({ parent, url }) {
  let { userPromise } = await parent();
  let user = await userPromise;

  if (!user) {
    error(401, { message: 'This page requires authentication', loginNeeded: true });
  }

  let target = resolve('/users/[user_id]', { user_id: user.login });
  let queryString = url.searchParams.toString();
  if (queryString) {
    target += `?${queryString}`;
  }

  redirect(308, target);
}
