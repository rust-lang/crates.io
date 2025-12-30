<script lang="ts">
  import type { PaginationState } from '$lib/utils/pagination';

  import { page } from '$app/state';
  import { SvelteURLSearchParams } from 'svelte/reactivity';

  import LeftPagIcon from '$lib/assets/left-pag.svg?component';
  import RightPagIcon from '$lib/assets/right-pag.svg?component';
  import Tooltip from '$lib/components/Tooltip.svelte';

  interface Props {
    pagination: PaginationState;
  }

  let { pagination }: Props = $props();

  function buildPageUrl(pageNum: number): string {
    let params = new SvelteURLSearchParams(page.url.searchParams);
    params.set('page', String(pageNum));
    return `?${params.toString()}`;
  }
</script>

<nav class="pagination" aria-label="Pagination navigation">
  {#if pagination.currentPage === 1}
    <span class="prev disabled" data-test-pagination-prev>
      <LeftPagIcon />
    </span>
  {:else}
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolve() doesn't support query params -->
    <a href={buildPageUrl(pagination.prevPage)} class="prev" rel="prev" title="previous page" data-test-pagination-prev>
      <LeftPagIcon />
    </a>
  {/if}

  <!-- eslint-disable svelte/no-navigation-without-resolve -- resolve() doesn't support query params -->
  <ol>
    {#each pagination.pages as pageNum (pageNum)}
      <li>
        <a
          href={buildPageUrl(pageNum)}
          class:active={pageNum === pagination.currentPage}
          title={`Go to page ${pageNum}`}
        >
          {pageNum}
        </a>
      </li>
    {/each}
  </ol>
  <!-- eslint-enable svelte/no-navigation-without-resolve -->

  {#if pagination.currentPage === pagination.availablePages}
    <span class="next disabled" data-test-pagination-next>
      <RightPagIcon />
      {#if pagination.maxPages && pagination.currentPage === pagination.maxPages}
        <Tooltip>
          For performance reasons, no more pages are available. For bulk data access, please visit
          <a href="https://crates.io/data-access" target="_blank" rel="noopener noreferrer">crates.io/data-access</a>.
        </Tooltip>
      {/if}
    </span>
  {:else}
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolve() doesn't support query params -->
    <a href={buildPageUrl(pagination.nextPage)} class="next" rel="next" title="next page" data-test-pagination-next>
      <RightPagIcon />
    </a>
  {/if}
</nav>

<style>
  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 110%;
    margin-bottom: var(--space-xs);
  }

  ol {
    list-style: none;
    padding: 0;
    margin: 0;
    display: inline-block;
  }

  li {
    display: inline-block;

    &:not(:first-child) {
      margin-left: var(--space-3xs);
    }
  }

  a,
  .disabled {
    color: var(--main-color-light);
    text-decoration: none;
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--space-3xs);
  }

  a:hover {
    background-color: var(--main-bg-dark);
  }

  a.active {
    background-color: var(--main-bg-dark);
  }

  .pagination :global(img),
  .pagination :global(svg) {
    vertical-align: middle;
    width: 2em;
    height: 2em;
  }

  .prev :global(circle),
  .next :global(circle) {
    fill: none;
  }

  .prev :global(path),
  .next :global(path) {
    fill: currentColor;
  }

  .prev:hover:not(.disabled) :global(circle),
  .next:hover:not(.disabled) :global(circle) {
    fill: var(--main-bg-dark);
  }

  .next:hover,
  .prev:hover {
    background: none;
  }

  .next.disabled,
  .prev.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
