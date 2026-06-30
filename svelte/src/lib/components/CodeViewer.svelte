<script lang="ts">
  import type { CodeViewOptions, SelectedLineRange } from '@pierre/diffs';
  import type { WorkerPoolManager } from '@pierre/diffs/worker';

  import { onMount, untrack } from 'svelte';
  import { CodeView } from '@pierre/diffs';
  import { getOrCreateWorkerPoolSingleton } from '@pierre/diffs/worker';
  import WorkerUrl from '@pierre/diffs/worker/worker.js?worker&url';

  import { registerCustomExtensions } from '$lib/utils/syntax-language';

  interface Props {
    content: { path: string; text: string; meta: string; cacheKey: string } | null;
    colorScheme: 'light' | 'dark';
    lineHash?: string;
    onLineHashChange?: (hash: string) => void;
  }

  let { content, colorScheme, lineHash = '', onLineHashChange }: Props = $props();

  const THEMES = { light: 'github-light', dark: 'github-dark' } as const;

  const LINE_HASH_PATTERN = /^L([1-9]\d*)(?:-L([1-9]\d*))?$/;

  let container = $state.raw<HTMLElement>();
  let view = $state.raw<CodeView>();
  let selectedRange = $derived(parseLineHash(lineHash));

  function options(): CodeViewOptions<undefined> {
    return {
      theme: THEMES,
      themeType: colorScheme,
      overflow: 'scroll',
      layout: {
        paddingTop: 0,
        paddingBottom: 0,
        gap: 0,
      },
      enableLineSelection: true,
      onLineSelectionEnd: selection => {
        let hash = formatLineHash(selection) ?? '';
        onLineHashChange?.(hash);
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

  function parseLineHash(hash: string): SelectedLineRange | null {
    let match = LINE_HASH_PATTERN.exec(hash.startsWith('#') ? hash.slice(1) : hash);
    if (!match) return null;

    let start = Number.parseInt(match[1]!, 10);
    let end = match[2] ? Number.parseInt(match[2], 10) : start;

    return { start, end };
  }

  function formatLineHash(range: SelectedLineRange | null): string | null {
    if (!range) return null;

    let start = Math.min(range.start, range.end);
    let end = Math.max(range.start, range.end);
    return start === end ? `#L${start}` : `#L${start}-L${end}`;
  }

  onMount(() => {
    registerCustomExtensions();
    view = new CodeView(options(), getHighlighterPool());
    view.setup(container!);
    return () => view?.cleanUp();
  });

  $effect(() => view?.setOptions(options()));

  $effect(() => {
    let items = [];
    if (content) {
      let file = {
        name: content.path,
        contents: content.text,
        cacheKey: content.cacheKey,
      };
      items.push({ id: content.path, type: 'file' as const, file });
    }
    view?.setItems(items);

    // Render immediately to avoid `scrollTo()` resolution warnings
    view?.render(true);

    if (content) {
      let range = untrack(() => selectedRange);
      if (range) {
        view?.scrollTo({ type: 'range', id: content.path, range });
      } else {
        view?.scrollTo({ type: 'position', position: 0, behavior: 'instant' });
      }
    }
  });

  $effect(() => {
    if (content && selectedRange) {
      view?.setSelectedLines({ id: content.path, range: selectedRange }, { notify: false });
    } else {
      view?.clearSelectedLines({ notify: false });
    }
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
