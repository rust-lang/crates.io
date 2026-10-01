<script lang="ts">
  import { getSession } from '#lib/utils/session.svelte.ts';

  let { children } = $props();

  let session = getSession();
</script>

{#await session.initialPromise then}
  {@render children()}
{/await}
