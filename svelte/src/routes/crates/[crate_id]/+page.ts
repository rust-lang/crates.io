import { loadReadme } from '$lib/utils/readme';

export async function load({ fetch, parent }) {
  let { crate, defaultVersion } = await parent();

  let readmePromise = loadReadme(fetch, crate.name, defaultVersion.num);

  return { readmePromise };
}
