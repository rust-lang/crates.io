<script lang="ts">
  import PageHeader from '$lib/components/PageHeader.svelte';

  let { data } = $props();
</script>

<svelte:head>
  <title>Category Slugs - crates.io</title>
</svelte:head>

<PageHeader title="All Valid Category Slugs" />

<dl class="list">
  {#each data.categorySlugs as category (category.id)}
    <dt data-test-category-slug={category.slug}>{category.slug}</dt>
    <dd data-test-category-description={category.slug}>{category.description}</dd>
  {/each}
</dl>

<style>
  .list {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
    margin-bottom: var(--space-s);
  }

  .list dt {
    padding: var(--space-s);
    padding-bottom: var(--space-2xs);
    font-family: var(--font-monospace);
    font-weight: bold;
  }

  .list dd {
    margin: 0 var(--space-s) var(--space-s);
  }

  .list > :global(* + dt) {
    border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #424242);
  }
</style>
