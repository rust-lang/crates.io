<script lang="ts">
  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import Row from '$lib/components/dependency-list/Row.svelte';

  let { data } = $props();

  let normal = $derived(data.dependencies.filter(d => d.kind === 'normal'));
  let build = $derived(data.dependencies.filter(d => d.kind === 'build'));
  let dev = $derived(data.dependencies.filter(d => d.kind === 'dev'));
  let descriptions = $derived(data.descriptionMap);
</script>

<svelte:head>
  <title>{data.crate.name} - crates.io: Rust Package Registry</title>
</svelte:head>

<CrateHeader crate={data.crate} version={data.version} versionNum={data.version.num} />

<h2 class="heading">Dependencies</h2>
{#if normal.length > 0}
  <ul class="list" data-test-dependencies>
    {#each normal as dependency (dependency.id)}
      <li><Row {dependency} descriptionPromise={descriptions.get(dependency.crate_id)} /></li>
    {/each}
  </ul>
{:else}
  <div data-test-no-dependencies>
    This version of the "{data.crate.name}" crate has no dependencies
  </div>
{/if}

{#if build.length > 0}
  <h2 class="heading">Build-Dependencies</h2>
  <ul class="list" data-test-build-dependencies>
    {#each build as dependency (dependency.id)}
      <li><Row {dependency} descriptionPromise={descriptions.get(dependency.crate_id)} /></li>
    {/each}
  </ul>
{/if}

{#if dev.length > 0}
  <h2 class="heading">Dev-Dependencies</h2>
  <ul class="list" data-test-dev-dependencies>
    {#each dev as dependency (dependency.id)}
      <li><Row {dependency} descriptionPromise={descriptions.get(dependency.crate_id)} /></li>
    {/each}
  </ul>
{/if}

<style>
  .list {
    list-style: none;
    margin: 0;
    padding: 0;

    > * + * {
      margin-top: var(--space-2xs);
    }
  }

  .heading {
    font-size: 1.17em;
    margin-block-start: 1em;
    margin-block-end: 1em;
  }
</style>
