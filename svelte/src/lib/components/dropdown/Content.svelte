<script lang="ts">
  import type { Placement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import Popover from '../Popover.svelte';
  import { getDropdown } from './context';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    placement?: Placement;
    children: Snippet;
  }

  let props: Props = $props();

  let dropdown = getDropdown();
</script>

<Popover
  bind:open={dropdown.isExpanded}
  anchor={dropdown.trigger}
  minWidthFromAnchor
  id={dropdown.contentId}
  {...props}
/>
