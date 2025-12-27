<script lang="ts">
  import { resolve } from '$app/paths';

  import PageHeader from '$lib/components/PageHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import { calculatePagination } from '$lib/utils/pagination';

  let { data } = $props();

  let pagination = $derived(calculatePagination(data.page, data.perPage, data.keywords.meta.total));
</script>

<svelte:head>
  <title>Keywords - crates.io</title>
</svelte:head>

<PageHeader title="All Keywords" />

<div class="results-meta">
  <ResultsCount
    start={pagination.currentPageStart}
    end={pagination.currentPageEnd}
    total={data.keywords.meta.total}
    data-test-keywords-nav
  />

  <!-- TODO: Add SortDropdown component when available -->
</div>

<div class="list">
  {#each data.keywords.keywords as keyword (keyword.id)}
    <div class="row" data-test-keyword={keyword.id}>
      <a href={resolve('/keywords/[keyword_id]', { keyword_id: keyword.id })}>{keyword.id}</a>
      <span class="text--small" data-test-count>
        {keyword.crates_cnt.toLocaleString()}
        {keyword.crates_cnt === 1 ? 'crate' : 'crates'}
      </span>
    </div>
  {/each}
</div>

<Pagination {pagination} />

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
</style>
