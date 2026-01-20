import { loadPlaygroundCrates } from '$lib/utils/playground';

export const ssr = false;

export async function load({ fetch }) {
  return {
    playgroundCratesPromise: loadPlaygroundCrates(fetch),
  };
}
