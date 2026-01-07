<script lang="ts">
  import CrateList from '$lib/components/CrateList.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import { calculatePagination } from '$lib/utils/pagination';

  let { data } = $props();

  let pagination = $derived(
    calculatePagination(data.page, data.perPage, data.cratesResponse.meta.total, data.maxPages),
  );

  let currentSortBy = $derived.by(() => {
    switch (data.sort) {
      case 'alpha':
        return 'Alphabetical';
      case 'downloads':
        return 'All-Time Downloads';
      case 'recent-updates':
        return 'Recent Updates';
      case 'new':
        return 'Newly Added';
      default:
        return 'Recent Downloads';
    }
  });
</script>

<svelte:head>
  <title>{data.keyword} - Keywords - crates.io</title>
</svelte:head>

<PageHeader title="All Crates" suffix="for keyword '{data.keyword}'" />

<div class="results-meta">
  <ResultsCount
    start={pagination.currentPageStart}
    end={pagination.currentPageEnd}
    total={data.cratesResponse.meta.total}
    data-test-keyword-nav
  />

  <div data-test-keyword-sort class="sort-by-v-center">
    <span class="text--small">Sort by</span>
    <SortDropdown.Root current={currentSortBy}>
      <SortDropdown.Option query={{ sort: 'alpha' }}>Alphabetical</SortDropdown.Option>
      <SortDropdown.Option query={{ sort: 'downloads' }}>All-Time Downloads</SortDropdown.Option>
      <SortDropdown.Option query={{ sort: 'recent-downloads' }}>Recent Downloads</SortDropdown.Option>
      <SortDropdown.Option query={{ sort: 'recent-updates' }}>Recent Updates</SortDropdown.Option>
      <SortDropdown.Option query={{ sort: 'new' }}>Newly Added</SortDropdown.Option>
    </SortDropdown.Root>
  </div>
</div>

<CrateList crates={data.cratesResponse.crates} style="margin-bottom: var(--space-s);" />

<Pagination {pagination} />

<style>
  .results-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-s);
  }
</style>
