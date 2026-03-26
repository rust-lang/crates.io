<script lang="ts">
  import type { paths } from '@crates-io/api-client';

  import { onMount, untrack } from 'svelte';
  import { page } from '$app/state';
  import { createClient } from '@crates-io/api-client';

  import Alert from '$lib/components/Alert.svelte';
  import CrateList from '$lib/components/CrateList.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import { getSearchFormContext } from '$lib/search-form.svelte';
  import { calculatePagination } from '$lib/utils/pagination';
  import { hasMultiCategoryFilter, processSearchQuery } from '$lib/utils/search';

  const MAX_PAGES = 20;

  let { data } = $props();

  let searchFormContext = getSearchFormContext();

  // Sync URL ?q= param to header search state
  $effect(() => {
    searchFormContext.value = page.url.searchParams.get('q') ?? '';
  });
  onMount(() => () => {
    searchFormContext.value = '';
  });

  type CratesResponse = paths['/api/v1/crates']['get']['responses']['200']['content']['application/json'];

  // Tracks the last successful response, persisting across failed requests
  let cratesResponse: CratesResponse | undefined = $state();
  let lastCompletedHadError: boolean = $state(false);
  let loading: boolean = $state(false);
  let hasCompleted: boolean = $state(false);

  let client = createClient({ fetch });
  let abortController: AbortController | undefined;

  async function fetchData() {
    // Cancel any in-flight request
    abortController?.abort();
    abortController = new AbortController();
    let { signal } = abortController;

    loading = true;

    let query = data.q.trim();
    let searchParams = data.allKeywords
      ? { page: data.page, per_page: data.perPage, sort: data.sort, q: query, all_keywords: data.allKeywords }
      : { page: data.page, per_page: data.perPage, sort: data.sort, ...processSearchQuery(query) };

    try {
      let response = await client.GET('/api/v1/crates', { params: { query: searchParams }, signal });

      if (signal.aborted) return;

      if (response.error) {
        throw new Error('Failed to load search results');
      }

      cratesResponse = response.data;
      lastCompletedHadError = false;
    } catch {
      if (signal.aborted) return;
      lastCompletedHadError = true;
    } finally {
      if (!signal.aborted) {
        loading = false;
        hasCompleted = true;
      }
    }
  }

  let firstResultPending = $derived(!hasCompleted && loading);

  $effect(() => {
    // Re-fetch when any query param changes
    void data.q;
    void data.page;
    void data.sort;
    void data.allKeywords;

    untrack(() => fetchData());
  });

  let pagination = $derived(
    cratesResponse ? calculatePagination(data.page, data.perPage, cratesResponse.meta.total, MAX_PAGES) : undefined,
  );

  let currentSortBy = $derived.by(() => {
    switch (data.sort) {
      case 'downloads':
        return 'All-Time Downloads';
      case 'recent-downloads':
        return 'Recent Downloads';
      case 'recent-updates':
        return 'Recent Updates';
      case 'new':
        return 'Newly Added';
      default:
        return 'Relevance';
    }
  });

  let searchTitle = $derived('Search Results' + (data.q ? ` for '${data.q}'` : ''));
</script>

<PageTitle title={searchTitle} />

<PageHeader
  title="Search Results"
  suffix={data.q ? `for '${data.q}'` : undefined}
  showSpinner={loading}
  data-test-header
/>

{#if hasMultiCategoryFilter(data.q)}
  <Alert variant="warning">
    Support for using multiple <code>category:</code> filters is not yet implemented.
  </Alert>
{/if}

{#if firstResultPending}
  <h2>Loading search results...</h2>
{:else if lastCompletedHadError}
  <p data-test-error-message>
    Unfortunately something went wrong while loading the search results. Feel free to try again, or let the
    <a href="mailto:help@crates.io">crates.io team</a>
    know if the problem persists.
  </p>

  <button
    type="button"
    disabled={loading}
    class="try-again-button button"
    data-test-try-again-button
    onclick={fetchData}
  >
    Try Again
  </button>
{:else if cratesResponse && cratesResponse.meta.total > 0 && pagination}
  <div class="results-meta">
    <ResultsCount
      start={pagination.currentPageStart}
      end={pagination.currentPageEnd}
      total={cratesResponse.meta.total}
      data-test-search-nav
    />

    <div data-test-search-sort class="sort-by-v-center">
      <span class="text--small">Sort by</span>
      <SortDropdown.Root current={currentSortBy}>
        <SortDropdown.Option query={{ page: '1', sort: 'relevance' }}>Relevance</SortDropdown.Option>
        <SortDropdown.Option query={{ page: '1', sort: 'downloads' }}>All-Time Downloads</SortDropdown.Option>
        <SortDropdown.Option query={{ page: '1', sort: 'recent-downloads' }}>Recent Downloads</SortDropdown.Option>
        <SortDropdown.Option query={{ page: '1', sort: 'recent-updates' }}>Recent Updates</SortDropdown.Option>
        <SortDropdown.Option query={{ page: '1', sort: 'new' }}>Newly Added</SortDropdown.Option>
      </SortDropdown.Root>
    </div>
  </div>

  <CrateList crates={cratesResponse.crates} style="margin-bottom: var(--space-s);" />

  <Pagination {pagination} />
{:else}
  <h2>
    0 crates found. <a href="https://doc.rust-lang.org/cargo/getting-started/">Get started</a> and create your own.
  </h2>
{/if}

<style>
  .results-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-s);
  }

  .try-again-button {
    align-self: center;
    margin-top: var(--space-m);
  }
</style>
