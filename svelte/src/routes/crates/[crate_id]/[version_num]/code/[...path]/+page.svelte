<script lang="ts">
  import type { ManifestFile } from '$lib/utils/zip-archive';

  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import prettyBytes from 'pretty-bytes';

  import { getColorScheme } from '$lib/color-scheme.svelte';
  import CodeViewer from '$lib/components/CodeViewer.svelte';
  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import FileTree from '$lib/components/FileTree.svelte';
  import { loadFile } from '$lib/utils/zip-archive';

  type FileState =
    | { kind: 'loading' }
    | { kind: 'content'; file: ManifestFile; text: string }
    | { kind: 'binary' }
    | { kind: 'unavailable' }
    | { kind: 'error'; message: string };

  let { data } = $props();

  let crate = $derived(data.crate);
  let version = $derived(data.version);
  let manifest = $derived(data.manifest);
  let cdnBase = $derived(data.cdnBase);
  let selectedPath = $derived(data.selectedPath);

  let filesByPath = $derived(new Map((manifest?.files ?? []).map(file => [file.path, file])));

  let colorScheme = getColorScheme();

  let fileState = $state<FileState>({ kind: 'loading' });

  let content = $derived.by(() => {
    if (fileState.kind !== 'content') return null;
    return {
      path: fileState.file.path,
      text: fileState.text,
      meta: prettyBytes(fileState.file.uncompressed_size, { binary: true }),
    };
  });

  $effect(() => {
    let path = selectedPath;
    if (!path) return;

    void showFile(path);
  });

  async function showFile(path: string) {
    let file = filesByPath.get(path);
    if (!file) {
      fileState = { kind: 'error', message: `File "${path}" was not found in this archive.` };
      return;
    }

    try {
      let result = await loadFile(fetch, cdnBase, crate.name, version.num, file);

      // Drop a stale result if a newer navigation superseded this
      // load while it was in flight.
      if (selectedPath !== path) return;

      if (result === null) {
        fileState = { kind: 'unavailable' };
      } else if (result.kind === 'binary') {
        fileState = { kind: 'binary' };
      } else {
        fileState = { kind: 'content', file, text: result.text };
      }
    } catch (error) {
      if (selectedPath !== path) return;

      let message = error instanceof Error ? error.message : String(error);
      fileState = { kind: 'error', message };
    }
  }

  function navigateTo(path: string) {
    if (!filesByPath.has(path)) return;

    let href = resolve('/crates/[crate_id]/[version_num]/code/[...path]', {
      crate_id: crate.id,
      version_num: version.num,
      path,
    });

    void goto(href, { keepFocus: true, noScroll: true });
  }
</script>

{#snippet unavailableMessage()}
  <div class="unavailable" data-test-archive-unavailable>
    <p>The source code for this version is not available yet.</p>
    <p>
      Archives are built shortly after a version is published. This usually only takes a few seconds, so please try
      again in a couple of minutes.
    </p>
  </div>
{/snippet}

<CrateHeader {crate} {version} versionNum={version.num} keywords={data.keywords} ownersPromise={data.ownersPromise} />

{#if !manifest}
  {@render unavailableMessage()}
{:else}
  <div class="viewer">
    <aside class="tree-panel" aria-label="File tree">
      <FileTree
        paths={manifest.files.map(file => file.path)}
        {selectedPath}
        onselect={navigateTo}
        colorScheme={colorScheme.resolvedScheme}
      />
    </aside>

    <section class="code-panel" aria-label="File contents">
      {#if fileState.kind === 'unavailable'}
        {@render unavailableMessage()}
      {:else if fileState.kind === 'binary'}
        <div class="message" data-test-binary-file>
          <p>This file is binary and can't be displayed.</p>
        </div>
      {:else if fileState.kind === 'error'}
        <div class="error" data-test-load-error>Failed to load file: {fileState.message}</div>
      {/if}

      <CodeViewer {content} colorScheme={colorScheme.resolvedScheme} />
    </section>
  </div>
{/if}

<style>
  .unavailable,
  .message {
    padding: var(--space-m);
    color: var(--main-color-light);
    line-height: 1.4;

    p {
      margin: 0 0 var(--space-2xs);
    }
  }

  .error {
    padding: var(--space-s);
    color: light-dark(oklch(0.5 0.15 24), oklch(0.8 0.07 24));
  }

  .viewer {
    display: grid;
    grid-template-columns: minmax(200px, 280px) 1fr;
    gap: var(--space-s);
    height: 70vh;
    min-height: 400px;
  }

  .tree-panel {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);
    overflow: hidden;
  }

  .code-panel {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);
    overflow: hidden;
  }

  @media only screen and (max-width: 750px) {
    .viewer {
      grid-template-columns: 1fr;
      height: auto;
    }

    .tree-panel {
      height: 240px;
    }

    .code-panel {
      height: 60vh;
    }
  }
</style>
