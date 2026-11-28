import type { components } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';

type Version = components['schemas']['Version'];

// A smaller first page fills the list sooner.
const PER_PAGE_FIRST = 25;
const PER_PAGE = 50;

/** Loads the versions of a crate in semver order, one page at a time. */
export class VersionStore {
  #crateName: string;
  #nextPage: string | null = $state('?sort=semver');
  #request: Promise<void> | null = null;

  versions: Version[] = $state.raw([]);
  isLoading = $state(false);
  error = $state(false);

  constructor(crateName: string) {
    this.#crateName = crateName;
  }

  get isComplete(): boolean {
    return this.#nextPage === null;
  }

  /** Loads the next page, unless a request is already running or all versions are loaded. */
  loadMore(): Promise<void> {
    if (this.#nextPage === null) return Promise.resolve();

    let perPage = this.versions.length === 0 ? PER_PAGE_FIRST : PER_PAGE;
    this.#request ??= this.#load(this.#nextPage, perPage).finally(() => (this.#request = null));
    return this.#request;
  }

  async #load(nextPage: string, perPage: number) {
    this.isLoading = true;
    this.error = false;
    try {
      let client = createClient();
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      let params = Object.fromEntries(new URLSearchParams(nextPage)) as Record<string, string>;
      let query = { ...params, per_page: perPage };
      let response = await client.GET('/api/v1/crates/{name}/versions', {
        params: { path: { name: this.#crateName }, query },
      });

      if (response.error) {
        this.error = true;
      } else {
        this.versions = [...this.versions, ...response.data.versions];
        this.#nextPage = response.data.meta.next_page ?? null;
      }
    } catch {
      this.error = true;
    } finally {
      this.isLoading = false;
    }
  }
}
