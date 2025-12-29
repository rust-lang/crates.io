<script lang="ts">
  import type { Snippet } from 'svelte';

  import { SearchFormContext, setSearchFormContext } from '$lib/search-form.svelte';

  let { children }: { children: Snippet } = $props();

  let searchFormContext = new SearchFormContext();
  setSearchFormContext(searchFormContext);
</script>

{@render children()}
