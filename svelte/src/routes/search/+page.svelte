<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';

  import { getSearchFormContext } from '$lib/search-form.svelte';

  let searchFormContext = getSearchFormContext();

  // Sync URL ?q= param to header search state
  $effect(() => {
    searchFormContext.value = page.url.searchParams.get('q') ?? '';
  });
  onMount(() => () => {
    searchFormContext.value = '';
  });
</script>

<h1>Search</h1>
<p>Stub route for /search</p>
