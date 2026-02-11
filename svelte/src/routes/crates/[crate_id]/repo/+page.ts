import { redirect } from '@sveltejs/kit';

export async function load({ parent }) {
  let { crate } = await parent();

  if (crate.repository) {
    redirect(302, crate.repository);
  }
}
