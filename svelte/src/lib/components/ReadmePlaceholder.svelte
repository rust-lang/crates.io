<script lang="ts">
  import Placeholder from '$lib/components/Placeholder.svelte';

  let radius = 'var(--space-3xs)';
  let text = { width: '100%', height: '16px', opacity: 0.3, style: 'margin-top: var(--space-xs)' };
</script>

<div>
  <!-- eslint-disable svelte/require-each-key -->

  <Placeholder width="30%" height="25px" {radius} opacity={0.6} style="margin: var(--space-s) 0 var(--space-m)" />
  {#each { length: 5 } as _}
    <Placeholder {...text} {radius} />
  {/each}

  <Placeholder width="50%" height="20px" {radius} opacity={0.6} style="margin: var(--space-l) 0 var(--space-m)" />
  {#each { length: 3 } as _}
    <Placeholder {...text} {radius} />
  {/each}
</div>
