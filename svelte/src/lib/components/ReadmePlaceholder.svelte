<script lang="ts">
  import Placeholder from '$lib/components/Placeholder.svelte';

  let radius = 'var(--space-3xs)';
  let text = { width: '100%', height: '16px', opacity: 0.3, class: 'mt-xs' };
</script>

<div>
  <!-- eslint-disable svelte/require-each-key -->

  <Placeholder width="30%" height="25px" {radius} opacity={0.6} class="mt-s mb-m" />
  {#each { length: 5 }}
    <Placeholder {...text} {radius} />
  {/each}

  <Placeholder width="50%" height="20px" {radius} opacity={0.6} class="mt-l mb-m" />
  {#each { length: 3 }}
    <Placeholder {...text} {radius} />
  {/each}
</div>
