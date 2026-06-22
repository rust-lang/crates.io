<script lang="ts">
  import { onMount } from 'svelte';
  import { File as FileView } from '@pierre/diffs';

  import { languageForPath } from '$lib/utils/syntax-language';

  interface Props {
    content: { path: string; text: string; meta: string | null } | null;
    colorScheme: 'light' | 'dark';
  }

  let { content, colorScheme }: Props = $props();

  const THEMES = { light: 'github-light', dark: 'github-dark' } as const;

  let container = $state.raw<HTMLElement>();
  let view = $state.raw<FileView>();

  onMount(() => {
    view = new FileView({
      theme: THEMES,
      themeType: colorScheme,
      overflow: 'wrap',
      renderHeaderMetadata: () => content?.meta ?? null,
    });
    return () => view?.cleanUp();
  });

  $effect(() => view?.setThemeType(colorScheme));

  $effect(() => {
    if (!container || !content) return;
    view?.render({
      file: { name: content.path, contents: content.text, lang: languageForPath(content.path) },
      containerWrapper: container,
    });
  });
</script>

<div class="code" class:hidden={content === null} bind:this={container} data-test-code-viewer></div>

<style>
  .hidden {
    display: none;
  }

  .code {
    flex: 1;
    min-height: 0;
    overflow: auto;
    font-size: calc(0.85 * var(--space-s));
    background-color: light-dark(white, #141413);
  }

  .code :global(diffs-container) {
    --diffs-font-family: var(--font-monospace);
    --diffs-header-font-family: var(--font-body);
    --diffs-light-bg: white;
    --diffs-dark-bg: #141413;
  }
</style>
