<script lang="ts">
  import PageTitle from '$lib/components/PageTitle.svelte';

  let { data, children } = $props();
</script>

<svelte:head>
  {#if data.crate.description}
    <meta name="description" content={data.crate.description} />
  {/if}
</svelte:head>

<PageTitle title={data.crate.name} />

{@render children()}
