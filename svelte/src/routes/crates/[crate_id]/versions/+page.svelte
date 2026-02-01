<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { createClient } from '@crates-io/api-client';
  import { format } from 'date-fns';

  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import Row from '$lib/components/version-list/Row.svelte';

  type Version = components['schemas']['Version'];

  let { data } = $props();

  let isLoading = $state(false);

  // Client-side state for extra loaded versions and pagination
  let extraVersions = $state<Version[]>([]);
  let extraNextPage = $state<string | null>(null);

  // Reset client-side state when sort changes
  $effect(() => {
    void data.sort;

    extraVersions = [];
    extraNextPage = null;
  });

  // Use SSR data for `next_page` until we've loaded more, then use our tracked value
  let nextPage = $derived(extraVersions.length > 0 ? extraNextPage : data.nextPage);

  let versions = $derived([...data.versions, ...extraVersions]);
  let releaseTrackHighest = $derived(new Set(Object.values(data.releaseTracks).map(it => it.highest)));

  let currentSortBy = $derived(data.sort === 'semver' ? 'SemVer' : 'Date');

  function isHighestOfReleaseTrack(version: Version): boolean {
    return releaseTrackHighest.has(version.num);
  }

  async function loadMore() {
    if (!nextPage || isLoading) return;

    isLoading = true;
    try {
      let client = createClient({ fetch });

      let params = new URLSearchParams(nextPage);

      let response = await client.GET('/api/v1/crates/{name}/versions', {
        params: {
          path: { name: data.crate.name },
          query: Object.fromEntries(params.entries()) as Record<string, string>,
        },
      });

      if (!response.error) {
        extraVersions = [...extraVersions, ...response.data.versions];
        extraNextPage = response.data.meta.next_page ?? null;
      }
    } finally {
      isLoading = false;
    }
  }
</script>

<svelte:head>
  <title>{data.crate.name} - Versions - crates.io</title>
</svelte:head>

<CrateHeader crate={data.crate} />

<div class="results-meta">
  <span class="page-description text--small" data-test-page-description>
    <strong>{versions.length}</strong>
    of
    <strong>{data.crate.num_versions}</strong>
    <strong>{data.crate.name}</strong>
    versions since
    {format(data.crate.created_at, 'PPP')}
  </span>

  <div data-test-search-sort>
    <span class="sort-by-label">Sort by </span>
    <SortDropdown.Root current={currentSortBy}>
      <SortDropdown.Option query={{ sort: 'date' }} data-test-date-sort>Date</SortDropdown.Option>
      <SortDropdown.Option query={{ sort: 'semver' }} data-test-semver-sort>SemVer</SortDropdown.Option>
    </SortDropdown.Root>
  </div>
</div>

<ul class="list">
  {#each versions as version (version.id)}
    <li>
      <!-- TODO: pass isOwner once authenticated user loading is implemented -->
      <Row
        crateName={data.crate.name}
        {version}
        isHighestOfReleaseTrack={isHighestOfReleaseTrack(version)}
        isOwner={false}
        data-test-version={version.num}
      />
    </li>
  {/each}
</ul>

{#if isLoading || nextPage}
  <div class="load-more">
    <button
      type="button"
      class="load-more-button"
      data-test-id={isLoading ? 'loading' : 'load-more'}
      disabled={isLoading}
      onclick={loadMore}
    >
      {#if isLoading}
        Loading...<LoadingSpinner class="loading-spinner" />
      {:else}
        Load More
      {/if}
    </button>
  </div>
{/if}

<style>
  .results-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2xs);

    @media only screen and (max-width: 550px) {
      display: block;
    }
  }

  .page-description {
    @media only screen and (max-width: 550px) {
      display: block;
      margin-bottom: var(--space-s);
    }
  }

  .list {
    list-style: none;
    margin: var(--space-2xs) 0 var(--space-s);
    padding: 0;

    > * + * {
      margin-top: var(--space-2xs);
    }
  }

  .load-more {
    --shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);

    padding: 0 var(--space-m);

    button {
      border-radius: var(--space-3xs);
      box-shadow: var(--shadow);
      cursor: pointer;
      position: relative;
    }

    :global(.loading-spinner) {
      display: inline-flex;
      align-items: center;
      position: absolute;
      height: 100%;
      top: 0;
      margin-left: var(--space-2xs);
    }
  }
</style>
