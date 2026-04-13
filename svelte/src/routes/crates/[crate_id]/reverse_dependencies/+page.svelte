<script lang="ts">
  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ResultsCount from '$lib/components/ResultsCount.svelte';
  import Row from '$lib/components/rev-dep-list/Row.svelte';
  import { calculatePagination } from '$lib/utils/pagination';

  let { data } = $props();

  let pagination = $derived(calculatePagination(data.page, data.perPage, data.total));
</script>

<CrateHeader crate={data.crate} keywords={data.keywords} ownersPromise={data.ownersPromise} />

{#if data.total > 0}
  <div class="results-meta">
    <ResultsCount
      start={pagination.currentPageStart}
      end={pagination.currentPageEnd}
      total={data.total}
      name="reverse dependencies of {data.crate.name}"
    />
  </div>

  <ul class="list" data-test-list>
    {#each data.dependencies as dependency, index (dependency.id)}
      <li>
        <Row
          {dependency}
          descriptionPromise={data.descriptionMap.get(dependency.dependentCrateName)}
          data-test-row={index}
        />
      </li>
    {/each}
  </ul>

  <Pagination {pagination} />
{:else}
  <div class="no-results">This crate is not used as a dependency in any other crate on crates.io.</div>
{/if}

<style>
  .results-meta {
    margin-bottom: var(--space-s);
  }

  .list {
    list-style: none;
    margin: 0 0 var(--space-s);
    padding: 0;

    > * + * {
      margin-top: var(--space-2xs);
    }
  }

  .no-results {
    text-align: center;
    margin: var(--space-m) 0;
  }
</style>
