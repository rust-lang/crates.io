<script lang="ts">
  let { data, children } = $props();
</script>

<svelte:head>
  {#if data.crate.description}
    <meta name="description" content={data.crate.description} />
  {/if}
</svelte:head>

{@render children()}
