<script lang="ts">
  import type { Snippet } from 'svelte';

  import { page } from '$app/state';
  import { SvelteURLSearchParams } from 'svelte/reactivity';

  import * as Dropdown from '$lib/components/dropdown';

  interface Props {
    query: Record<string, string>;
    children: Snippet;
  }

  let { query, children }: Props = $props();

  let url = $derived.by(() => {
    let params = new SvelteURLSearchParams(page.url.searchParams);
    for (let [key, value] of Object.entries(query)) {
      params.set(key, value);
    }
    return `?${params.toString()}`;
  });
</script>

<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
<Dropdown.Item><a href={url}>{@render children()}</a></Dropdown.Item>
