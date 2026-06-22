<script lang="ts">
  import type { CodeViewOptions } from '@pierre/diffs';
  import type { WorkerPoolManager } from '@pierre/diffs/worker';

  import { onMount } from 'svelte';
  import { CodeView } from '@pierre/diffs';
  import { getOrCreateWorkerPoolSingleton } from '@pierre/diffs/worker';
  import WorkerUrl from '@pierre/diffs/worker/worker.js?worker&url';

  import { languageForPath } from '$lib/utils/syntax-language';

  interface Props {
    content: { path: string; text: string; meta: string | null } | null;
    colorScheme: 'light' | 'dark';
  }

  let { content, colorScheme }: Props = $props();

  const THEMES = { light: 'github-light', dark: 'github-dark' } as const;

  let container = $state.raw<HTMLElement>();
  let view = $state.raw<CodeView>();

  function options(): CodeViewOptions<undefined> {
    return {
      theme: THEMES,
      themeType: colorScheme,
      overflow: 'wrap',
      layout: {
        paddingTop: 0,
        paddingBottom: 0,
        gap: 0,
      },
      renderHeaderMetadata: () => content?.meta ?? null,
    };
  }

  function getHighlighterPool(): WorkerPoolManager {
    return getOrCreateWorkerPoolSingleton({
      poolOptions: {
        workerFactory: () => new Worker(WorkerUrl, { type: 'module' }),
        poolSize: 1,
      },
      highlighterOptions: {
        theme: THEMES,
        langs: ['rust', 'toml'],
      },
    });
  }

  onMount(() => {
    view = new CodeView(options(), getHighlighterPool());
    view.setup(container!);
    return () => view?.cleanUp();
  });

  $effect(() => view?.setOptions(options()));

  $effect(() => {
    let items = [];
    if (content) {
      let file = { name: content.path, contents: content.text, lang: languageForPath(content.path) };
      items.push({ id: content.path, type: 'file' as const, file });
    }
    view?.setItems(items);
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
