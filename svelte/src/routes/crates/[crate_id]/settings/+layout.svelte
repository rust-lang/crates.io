<script lang="ts">
  import { getSession } from '$lib/utils/session.svelte';

  let { children } = $props();

  let session = getSession();
</script>

<!-- Wait for `session.currentUser` to be set before rendering child routes. -->
{#await session.initialPromise then}
  {@render children()}
{/await}
