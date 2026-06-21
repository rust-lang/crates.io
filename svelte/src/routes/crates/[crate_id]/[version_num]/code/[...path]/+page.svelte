<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { File as FileView } from '@pierre/diffs';
  import { FileTree } from '@pierre/trees';
  import prettyBytes from 'pretty-bytes';

  import { getColorScheme } from '$lib/color-scheme.svelte';
  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import { languageForPath } from '$lib/utils/syntax-language';
  import { loadFile } from '$lib/utils/zip-archive';

  const THEMES = { light: 'github-light', dark: 'github-dark' } as const;

  // The mutually exclusive display states of the code panel.
  type FileState =
    | { kind: 'content' }
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

  let tree = $state.raw<FileTree>();
  let treeContainer = $state.raw<HTMLElement>();

  let fileView = $state.raw<FileView>();
  let fileContainer = $state.raw<HTMLElement>();

  let fileState = $state<FileState>({ kind: 'content' });

  // Canonical ancestor directory paths (trailing slash) for a file path,
  // e.g. `src/core/de.rs` -> [`src/`, `src/core/`]. Used to reveal a file in an
  // otherwise-collapsed tree.
  function ancestorDirectories(path: string): string[] {
    let segments = path.split('/').slice(0, -1);
    return segments.map((_, index) => segments.slice(0, index + 1).join('/') + '/');
  }

  onMount(() => {
    if (!manifest || !treeContainer || !fileContainer) return;

    tree = new FileTree({
      paths: manifest.files.map(file => file.path),
      initialExpansion: 'closed',
      initialExpandedPaths: selectedPath ? ancestorDirectories(selectedPath) : [],
      flattenEmptyDirectories: true,
      search: true,
      stickyFolders: true,
      onSelectionChange(paths) {
        let path = paths[0];
        if (path && path !== selectedPath && filesByPath.has(path)) {
          let href = resolve('/crates/[crate_id]/[version_num]/code/[...path]', {
            crate_id: crate.id,
            version_num: version.num,
            path,
          });
          void goto(href, { keepFocus: true, noScroll: true });
        }
      },
    });

    tree.render({ containerWrapper: treeContainer });

    fileView = new FileView({
      theme: THEMES,
      themeType: colorScheme.scheme,
      overflow: 'wrap',
      // Show the uncompressed file size in the built-in header. The callback
      // receives the file whose `name` is its archive path.
      renderHeaderMetadata: file => {
        let entry = filesByPath.get(file.name);
        return entry ? prettyBytes(entry.uncompressed_size, { binary: true }) : null;
      },
    });

    return () => {
      tree?.cleanUp();
      fileView?.cleanUp();
    };
  });

  // Load and display whichever file the URL points at and keep the tree's
  // selection in sync (so back/forward navigation highlights the right row).
  $effect(() => {
    let path = selectedPath;
    if (!path) return;

    void showFile(path);
    syncTreeSelection(path);
  });

  // Keep the code view and file tree in sync with the user's color scheme.
  $effect(() => {
    fileView?.setThemeType(colorScheme.scheme);

    let fileTreeContainer = tree?.getFileTreeContainer();
    if (fileTreeContainer) {
      fileTreeContainer.style.colorScheme = colorScheme.resolvedScheme;
    }
  });

  async function showFile(path: string) {
    if (!fileView || !fileContainer) return;

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
        return;
      }

      if (result.kind === 'binary') {
        fileState = { kind: 'binary' };
        return;
      }

      fileState = { kind: 'content' };

      fileView.render({
        file: { name: path, contents: result.text, lang: languageForPath(path) },
        containerWrapper: fileContainer,
      });
    } catch (error) {
      if (selectedPath !== path) return;

      let message = error instanceof Error ? error.message : String(error);
      fileState = { kind: 'error', message };
    }
  }

  function syncTreeSelection(path: string) {
    if (!tree) return;

    let current = tree.getSelectedPaths();
    if (current.length === 1 && current[0] === path) return;

    // Reveal the file by expanding its ancestor directories.
    // `scrollToPath()` only works once the target row is visible.
    for (let dir of ancestorDirectories(path)) {
      let item = tree.getItem(dir);
      if (item && 'expand' in item) item.expand();
    }

    for (let other of current) {
      tree.getItem(other)?.deselect();
    }
    tree.getItem(path)?.select();
    tree.scrollToPath(path, { focus: false, offset: 'center' });
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
      <div class="tree" bind:this={treeContainer}></div>
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

      <div
        class="code"
        class:hidden={fileState.kind !== 'content'}
        bind:this={fileContainer}
        data-test-code-viewer
      ></div>
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

  .hidden {
    display: none;
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

  .tree {
    height: 100%;
  }

  .tree :global(file-tree-container) {
    --trees-bg-override: light-dark(white, #141413);
    padding-top: var(--space-xs);
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
