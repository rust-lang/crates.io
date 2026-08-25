<script lang="ts">
  import CrateList from '$lib/components/CrateList.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import TeamPageHeader from '$lib/components/TeamPageHeader.svelte';
  import { calculatePagination } from '$lib/utils/pagination';

  const MAX_PAGES = 50;

  let { data } = $props();

  let pagination = $derived(calculatePagination(data.page, data.perPage, data.cratesResponse.meta.total, MAX_PAGES));

  let currentSortBy = $derived.by(() => {
    if (data.sort === 'downloads') return 'All-Time Downloads';
    if (data.sort === 'recent-downloads') return 'Recent Downloads';
    if (data.sort === 'recent-updates') return 'Recent Updates';
    if (data.sort === 'new') return 'Newly Added';
    return 'Alphabetical';
  });
</script>

<TeamPageHeader team={data.team} />

<div class="results-meta">
  <ResultsCount
    start={pagination.currentPageStart}
    end={pagination.currentPageEnd}
    total={data.cratesResponse.meta.total}
  />

  <div class="sort-by">
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
