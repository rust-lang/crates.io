<script lang="ts">
  import CrateList from '$lib/components/CrateList.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import { calculatePagination } from '$lib/utils/pagination';

  const MAX_PAGES = 20;

  let { data } = $props();

  let pagination = $derived(calculatePagination(data.page, data.perPage, data.cratesResponse.meta.total, MAX_PAGES));

  let suffix = $derived(data.letter ? `starting with '${data.letter}'` : undefined);

  let currentSortBy = $derived.by(() => {
    if (data.sort === 'downloads') return 'All-Time Downloads';
    if (data.sort === 'recent-downloads') return 'Recent Downloads';
    if (data.sort === 'recent-updates') return 'Recent Updates';
    if (data.sort === 'new') return 'Newly Added';
    return 'Alphabetical';
  });
</script>

<svelte:head>
  <title>Crates - crates.io</title>
</svelte:head>

<PageHeader title="All Crates" {suffix} />

<div class="results-meta">
  <ResultsCount
    start={pagination.currentPageStart}
    end={pagination.currentPageEnd}
    total={data.cratesResponse.meta.total}
    data-test-crates-nav
  />

  <div data-test-crates-sort class="sort-by-v-center">
    <span class="text--small">Sort by</span>
    <SortDropdown.Root current={currentSortBy}>
      <SortDropdown.Option query={{ page: '1', sort: 'alpha' }}>Alphabetical</SortDropdown.Option>
      <SortDropdown.Option query={{ page: '1', sort: 'downloads' }}>All-Time Downloads</SortDropdown.Option>
      <SortDropdown.Option query={{ page: '1', sort: 'recent-downloads' }}>Recent Downloads</SortDropdown.Option>
      <SortDropdown.Option query={{ page: '1', sort: 'recent-updates' }}>Recent Updates</SortDropdown.Option>
      <SortDropdown.Option query={{ page: '1', sort: 'new' }}>Newly Added</SortDropdown.Option>
    </SortDropdown.Root>
  </div>
</div>

<CrateList crates={data.cratesResponse.crates} style="margin-bottom: var(--space-s)" />

<Pagination {pagination} />

<style>
  .results-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-s);
  }
</style>
