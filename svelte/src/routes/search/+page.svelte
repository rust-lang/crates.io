<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';

  import Alert from '$lib/components/Alert.svelte';
  import CrateList from '$lib/components/CrateList.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import { getSearchFormContext } from '$lib/search-form.svelte';
  import { calculatePagination } from '$lib/utils/pagination';
  import { hasMultiCategoryFilter } from '$lib/utils/search';

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

  let pagination = $derived(calculatePagination(data.page, data.perPage, data.cratesResponse.meta.total, MAX_PAGES));

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

  let pageTitle = $derived('Search Results' + (data.q ? ` for '${data.q}'` : ''));
</script>

<svelte:head>
  <title>{pageTitle} - crates.io</title>
</svelte:head>

<PageHeader title="Search Results" suffix={data.q ? `for '${data.q}'` : undefined} data-test-header />

{#if hasMultiCategoryFilter(data.q)}
  <Alert variant="warning">
    Support for using multiple <code>category:</code> filters is not yet implemented.
  </Alert>
{/if}

{#if data.cratesResponse.meta.total > 0}
  <div class="results-meta">
    <ResultsCount
      start={pagination.currentPageStart}
      end={pagination.currentPageEnd}
      total={data.cratesResponse.meta.total}
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

  <CrateList crates={data.cratesResponse.crates} style="margin-bottom: var(--space-s);" />

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
</style>
