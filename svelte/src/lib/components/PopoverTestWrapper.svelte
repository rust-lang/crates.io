<script lang="ts">
  import type { Placement } from '@floating-ui/dom';

  import Popover from './Popover.svelte';

  interface Props {
    placement?: Placement;
  }

  let { placement }: Props = $props();

  let open = $state(false);
  let anchor = $state<HTMLButtonElement>();
</script>

<div style:padding="100px">
  <h1 data-test-heading style:margin-bottom="100px">
    <button type="button" data-test-trigger bind:this={anchor}>Toggle popover</button>

    <Popover bind:open {anchor} {placement} data-test-popover>
      <a href="#item" data-test-item>Item</a>
      <button type="button" data-test-close onclick={() => (open = false)}>Close</button>
    </Popover>
  </h1>

  <button type="button" data-test-outside>Outside</button>
  <button type="button" data-test-open onclick={() => (open = true)}>Open</button>
</div>
