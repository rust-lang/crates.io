<script lang="ts">
  import type { Snippet } from 'svelte';

  import { ColorSchemeState, setColorScheme } from '$lib/color-scheme.svelte';

  let { children }: { children: Snippet } = $props();

  let colorScheme = new ColorSchemeState();
  setColorScheme(colorScheme);

  $effect(() => {
    document.documentElement.dataset.colorScheme = colorScheme.resolvedScheme;
  });
</script>

{@render children()}
