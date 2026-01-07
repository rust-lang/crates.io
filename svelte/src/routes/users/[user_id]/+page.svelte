<script lang="ts">
  import GitHubIcon from '$lib/assets/github.svg?component';
  import CrateList from '$lib/components/CrateList.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import UserAvatar from '$lib/components/UserAvatar.svelte';
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

<svelte:head>
  <title>{data.user.login} - crates.io</title>
</svelte:head>

<PageHeader style="display: flex; align-items: center; gap: var(--space-xs);" data-test-heading>
  <UserAvatar user={{ ...data.user, kind: 'user' }} size="medium" data-test-avatar />
  <h1 data-test-username>{data.user.login}</h1>
  <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
  <a href={data.user.url} title={data.user.login} class="github-link" data-test-user-link>
    <GitHubIcon aria-label="GitHub profile" />
  </a>
</PageHeader>

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
  h1 {
    margin: 0;
  }

  .github-link {
    &,
    &:hover {
      color: var(--main-color);
    }

    :global(svg) {
      width: 32px;
      height: 32px;
    }
  }

  .results-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-s);
  }
</style>
