<script lang="ts">
  import type { ColorScheme } from '$lib/color-scheme.svelte';
  import type { Snippet } from 'svelte';

  import { ColorSchemeState, setColorScheme } from '$lib/color-scheme.svelte';

  let { children, theme }: { children: Snippet; theme?: ColorScheme } = $props();

  let colorScheme = new ColorSchemeState();
  setColorScheme(colorScheme);

  $effect(() => {
    if (theme) {
      colorScheme.setScheme(theme);
    }
    document.documentElement.dataset.colorScheme = colorScheme.resolvedScheme;
  });
</script>

{@render children()}
