<script lang="ts">
  import { onMount } from 'svelte';
  import { FILE_TREE_ICONS } from '@crates-io/file-tree-icons';
  import { FileTree as PierreFileTree } from '@pierre/trees';

  interface Props {
    paths: string[];
    selectedPath: string | null;
    onselect: (path: string) => void;
    colorScheme: 'light' | 'dark';
  }

  let { paths, selectedPath, onselect, colorScheme }: Props = $props();

  let container = $state.raw<HTMLElement>();
  let tree = $state.raw<PierreFileTree>();

  // Returns ancestor directory paths for a file path,
  // e.g. `src/core/de.rs` -> [`src/`, `src/core/`].
  function ancestorDirectories(path: string): string[] {
    let segments = path.split('/').slice(0, -1);
    return segments.map((_, index) => segments.slice(0, index + 1).join('/') + '/');
  }

  onMount(() => {
    if (!container) return;
    tree = new PierreFileTree({
      paths,
      icons: FILE_TREE_ICONS,
      initialExpansion: 'closed',
      initialExpandedPaths: selectedPath ? ancestorDirectories(selectedPath) : [],
      flattenEmptyDirectories: true,
      search: true,
      stickyFolders: true,
      onSelectionChange(p) {
        let path = p[0];
        if (path && path !== selectedPath) {
          onselect(path);
        }
      },
    });
    tree.render({ containerWrapper: container });
    return () => tree?.cleanUp();
  });

  $effect(() => {
    let path = selectedPath;
    if (!tree || !path) return;

    let current = tree.getSelectedPaths();
    if (current.length === 1 && current[0] === path) return;

    // `scrollToPath()` only works once the target row is visible.
    for (let dir of ancestorDirectories(path)) {
      let item = tree.getItem(dir);
      if (item && 'expand' in item) {
        item.expand();
      }
    }

    for (let other of current) {
      tree.getItem(other)?.deselect();
    }
    tree.getItem(path)?.select();
    tree.scrollToPath(path, { focus: false, offset: 'center' });
  });

  $effect(() => {
    let container = tree?.getFileTreeContainer();
    if (container) {
      container.style.colorScheme = colorScheme;
    }
  });
</script>

<div class="tree" bind:this={container}></div>

<style>
  .tree {
    height: 100%;
  }

  .tree :global(file-tree-container) {
    --trees-bg-override: light-dark(white, #141413);
    padding-top: var(--space-xs);
  }
</style>
