import { error } from '@sveltejs/kit';

import { fetchAdvisories } from '$lib/utils/rustsec';

export async function load({ fetch, params }) {
  let crateName = params.crate_id;

  try {
    let advisories = await fetchAdvisories(fetch, crateName);
    return { advisories };
  } catch {
    error(500, { message: `${crateName}: Failed to load advisories`, tryAgain: true });
  }
}
