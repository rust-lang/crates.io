<script lang="ts">
  import type { ResolvedPathname } from '$app/types';
  import type { VersionRowData } from './Row.svelte';

  import LoadingSpinner from '#lib/components/LoadingSpinner.svelte';
  import Row from './Row.svelte';

  interface Props {
    versions: VersionRowData[];
    isComplete: boolean;
    error: boolean;
    currentNum: string;
    defaultNum?: string;
    versionHref: (num: string) => ResolvedPathname;
    /** Called when the end of the list scrolls into view, or when the user retries after an error. */
    onLoadMore: () => void;
    /** Called when a version link is clicked. */
    onSelect?: () => void;
  }

  let { versions, isComplete, error, currentNum, defaultNum, versionHref, onLoadMore, onSelect }: Props = $props();

  function loadWhenVisible(loadingRow: HTMLElement) {
    let options = { root: loadingRow.parentElement, rootMargin: '100px' };
    let observer = new IntersectionObserver(entries => {
      let isVisible = entries.some(entry => entry.isIntersecting);
      if (isVisible) onLoadMore();
    }, options);
    observer.observe(loadingRow);
    return () => observer.disconnect();
  }
</script>

<div class="panel">
  <!-- Preloading on hover would load every version page that scrolls past the pointer. -->
  <ul class="list" data-sveltekit-preload-data="tap">
    {#each versions as version (version.num)}
      <li>
        <Row
          {version}
          href={versionHref(version.num)}
          isCurrent={version.num === currentNum}
          isDefault={version.num === defaultNum}
          onClick={onSelect}
        />
      </li>
    {/each}
  </ul>

  {#if error}
    <div class="status" data-test-error>
      Failed to load versions.
      <button type="button" class="button-reset retry" onclick={onLoadMore}>Try again</button>
    </div>
  {:else if !isComplete}
    <div class="status" {@attach loadWhenVisible} data-test-loading>
      <LoadingSpinner label={null} /> Loading…
    </div>
  {/if}
</div>

<style>
  /* Fills the remaining space of a flex column container, so the container can limit its height. */
  .panel {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .status {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2xs);
    padding: var(--space-xs);
    color: var(--main-color-light);
    font-size: 12px;
  }

  .retry {
    color: var(--link-color);
    text-decoration: underline;
    cursor: pointer;
  }
</style>
