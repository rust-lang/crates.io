<!--
  @component
  Button showing the current version, which opens a list of the versions in
  `store`. The list is a popover on wide screens and a fullscreen dialog below
  550px.
-->
<script lang="ts">
  import type { ResolvedPathname } from '$app/types';
  import type { VersionRowData } from './Row.svelte';
  import type { VersionStore } from './version-store.svelte';

  import { MediaQuery } from 'svelte/reactivity';

  import Dialog from '#lib/components/Dialog.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import Popover from '#lib/components/Popover.svelte';
  import Panel from './Panel.svelte';

  interface Props {
    store: Pick<VersionStore, 'isComplete' | 'error' | 'loadMore'> & { versions: VersionRowData[] };
    crateName: string;
    currentNum: string;
    defaultNum?: string;
    versionHref: (num: string) => ResolvedPathname;
  }

  let { store, crateName, currentNum, defaultNum, versionHref }: Props = $props();

  let isNarrow = new MediaQuery('max-width: 550px');

  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
</script>

<!-- The popover and the dialog both open themselves when the trigger is clicked. -->
<button
  bind:this={trigger}
  type="button"
  class="trigger"
  aria-haspopup="dialog"
  aria-expanded={open}
  data-test-version-switcher
>
  v{currentNum}<Icon class="i-mdi:menu-down" />
</button>

{#snippet panel()}
  <div class={['content', { popover: !isNarrow.current }]}>
    <Panel
      versions={store.versions}
      isComplete={store.isComplete}
      error={store.error}
      {currentNum}
      {defaultNum}
      {versionHref}
      onLoadMore={() => store.loadMore()}
      onSelect={() => (open = false)}
    />
  </div>
{/snippet}

{#if isNarrow.current}
  <Dialog bind:open title="{crateName} versions" fullscreen invoker={trigger}>
    {@render panel()}
  </Dialog>
{:else}
  <Popover bind:open anchor={trigger}>
    {@render panel()}
  </Popover>
{/if}

<style>
  .trigger {
    display: inline-flex;
    align-items: center;
    padding: 0 var(--space-3xs);
    border: 1px solid transparent;
    border-radius: var(--border-radius-small);
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;

    &:hover,
    &[aria-expanded='true'] {
      border-color: var(--gray-border);
      color: var(--main-color);
    }

    /* The icon has empty space on both sides of the triangle. This balances the right edge with the left one. */
    :global(.icon) {
      margin-right: -0.25em;
    }

    &[aria-expanded='true'] :global(.icon) {
      rotate: 180deg;
    }
  }

  .content {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .popover {
    width: 360px;
    max-height: min(480px, 70vh);
    border: 1px solid var(--gray-border);
    border-radius: var(--border-radius-small);
    background: var(--surface);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.15);
    overflow: hidden;
  }
</style>
