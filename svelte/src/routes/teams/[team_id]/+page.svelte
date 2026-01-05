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

  let pagination = $derived(calculatePagination(data.page, data.perPage, data.crates.meta.total, MAX_PAGES));

  // login format is "github:org_name:team_name"
  let orgName = $derived(data.team.login.split(':')[1]);

  let currentSortBy = $derived.by(() => {
    if (data.sort === 'downloads') return 'All-Time Downloads';
    if (data.sort === 'recent-downloads') return 'Recent Downloads';
    if (data.sort === 'recent-updates') return 'Recent Updates';
    if (data.sort === 'new') return 'Newly Added';
    return 'Alphabetical';
  });
</script>

<svelte:head>
  <title>{orgName}/{data.team.name} - crates.io</title>
</svelte:head>

<PageHeader style="display: flex; align-items: center;" data-test-heading>
  <UserAvatar
    user={{ ...data.team, kind: 'team' }}
    size="medium"
    style="margin-right: var(--space-m)"
    data-test-avatar
  />
  <div>
    <div class="header-row">
      <h1 data-test-org-name>{orgName}</h1>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
      <a href={data.team.url} title={data.team.login} class="github-link" data-test-github-link>
        <GitHubIcon aria-label="GitHub profile" />
      </a>
    </div>
    <h2 data-test-team-name>{data.team.name}</h2>
  </div>
</PageHeader>

<div class="results-meta">
  <ResultsCount start={pagination.currentPageStart} end={pagination.currentPageEnd} total={data.crates.meta.total} />

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

<CrateList crates={data.crates.crates} style="margin-bottom: var(--space-s)" />

<Pagination {pagination} />

<style>
  h1,
  h2 {
    margin: 0;
    padding: 0;
  }

  h2 {
    margin-top: var(--space-2xs);
    color: var(--main-color-light);
  }

  .header-row {
    display: flex;
    align-items: center;
  }

  .github-link {
    margin-left: var(--space-s);

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
