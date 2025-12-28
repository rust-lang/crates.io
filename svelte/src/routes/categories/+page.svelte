<script lang="ts">
  import { resolve } from '$app/paths';

  import PageHeader from '$lib/components/PageHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import * as SortDropdown from '$lib/components/sort-dropdown';
  import { calculatePagination } from '$lib/utils/pagination';

  let { data } = $props();

  let pagination = $derived(calculatePagination(data.page, data.perPage, data.categories.meta.total));
  let currentSortBy = $derived(data.sort === 'crates' ? '# Crates' : 'Alphabetical');
</script>

<svelte:head>
  <title>Categories - crates.io</title>
</svelte:head>

<PageHeader title="All Categories" />

<div class="results-meta">
  <ResultsCount
    start={pagination.currentPageStart}
    end={pagination.currentPageEnd}
    total={data.categories.meta.total}
    data-test-categories-nav
  />

  <div data-test-categories-sort class="sort-by-v-center">
    <span class="text--small">Sort by</span>
    <SortDropdown.Root current={currentSortBy}>
      <SortDropdown.Option query={{ sort: 'alpha' }}>Alphabetical</SortDropdown.Option>
      <SortDropdown.Option query={{ sort: 'crates' }}># Crates</SortDropdown.Option>
    </SortDropdown.Root>
  </div>
</div>

<div class="list">
  {#each data.categories.categories as category (category.id)}
    <div class="row" data-test-category={category.slug}>
      <div>
        <a href={resolve('/categories/[category_id]', { category_id: category.slug })}>{category.category}</a>
        <span class="text--small" data-test-crate-count>
          {category.crates_cnt.toLocaleString()}
          {category.crates_cnt === 1 ? 'crate' : 'crates'}
        </span>
      </div>
      <div class="description text--small">
        {category.description}
      </div>
    </div>
  {/each}
</div>

<Pagination {pagination} />

<div class="categories-footer">
  Want to categorize your crate?
  <a href="https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata">Add metadata!</a>
</div>

<style>
  .results-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-s);
  }

  .list {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
    margin-bottom: var(--space-s);
  }

  .list > * {
    padding: var(--space-s);
  }

  .list > * + * {
    border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #424242);
  }

  .description {
    margin-top: var(--space-2xs);
    line-height: 1.5;
  }

  .categories-footer {
    width: 100%;
    margin: var(--space-2xs) 0;
    text-align: center;
    font-size: 85%;
  }
</style>
