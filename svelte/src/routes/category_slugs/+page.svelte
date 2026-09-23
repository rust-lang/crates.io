<script lang="ts">
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import Panel from '$lib/components/Panel.svelte';

  let { data } = $props();
</script>

<PageTitle title="Category Slugs" />

<PageHeader title="All Valid Category Slugs" />

<Panel class="mb-s">
  <dl class="list">
    {#each data.categorySlugs as category (category.id)}
      <dt data-test-category-slug={category.slug}>{category.slug}</dt>
      <dd data-test-category-description={category.slug}>{category.description}</dd>
    {/each}
  </dl>
</Panel>

<style>
  .list {
    margin: 0;
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

  .list > dd + dt {
    border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #424242);
  }
</style>
