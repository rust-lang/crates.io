<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';

  import CrateList from '$lib/components/CrateList.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import { getSearchFormContext } from '$lib/search-form.svelte';
  import { calculatePagination } from '$lib/utils/pagination';

  let { data } = $props();

  let searchFormContext = getSearchFormContext();

  // Pre-fill search with category prefix (trailing space helps user not accidentally mangle the category)
  $effect(() => {
    searchFormContext.value = `category:${data.category.slug} `;
  });
  onMount(() => () => {
    searchFormContext.value = '';
  });

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
  <title>{data.category.category} - Categories - crates.io</title>
</svelte:head>

<PageHeader>
  <h1 class="heading">
    {#each data.category.parent_categories ?? [] as parent (parent.id)}
      <a href={resolve('/categories/[category_id]', { category_id: parent.slug })}>
        {parent.category}
      </a>::
    {/each}{data.category.category}
  </h1>
</PageHeader>

<div>
  <p>{data.category.description}</p>
</div>

{#if data.category.subcategories?.length}
  <div>
    <h2>Subcategories</h2>
    <div class="subcategories">
      {#each data.category.subcategories as subcategory (subcategory.id)}
        <div class="subcategory">
          <div>
            <a href={resolve('/categories/[category_id]', { category_id: subcategory.slug })}>
              {subcategory.category}
            </a>
            <span class="text--small">
              {subcategory.crates_cnt.toLocaleString()}
              {subcategory.crates_cnt === 1 ? 'crate' : 'crates'}
            </span>
          </div>
          <div class="category-description text--small">
            {subcategory.description}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}

<h2>Crates</h2>
<div class="results-meta">
  <ResultsCount
    start={pagination.currentPageStart}
    end={pagination.currentPageEnd}
    total={data.cratesResponse.meta.total}
    data-test-category-nav
  />

  <div data-test-category-sort>
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
  .heading {
    margin: 0;
  }

  .subcategories {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
    margin-bottom: var(--space-s);
  }

  .subcategories > :global(*) {
    padding: var(--space-s);
  }

  .subcategories > :global(* + *) {
    border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #424242);
  }

  .category-description {
    margin-top: var(--space-2xs);
  }

  .results-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-s);
  }
</style>
