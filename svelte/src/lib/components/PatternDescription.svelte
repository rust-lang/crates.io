<script lang="ts">
  let { pattern }: { pattern: string } = $props();
</script>

{#if pattern === '*'}
  Matches all crates on crates.io
{:else if pattern.endsWith('*')}
  Matches all crates starting with <strong>{pattern.slice(0, -1)}</strong>
{:else}
  Matches only the <strong>{pattern}</strong> crate
{/if}
