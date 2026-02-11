import { redirect } from '@sveltejs/kit';

export async function load({ parent }) {
  let { crate } = await parent();

  if (crate.documentation) {
    redirect(302, crate.documentation);
  }
}
