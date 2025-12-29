<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import type { DropdownContext } from './context';

  import { clickOutside } from '$lib/attachments/click-outside';
  import { setDropdown } from './context';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();
  let uniqueId = $props.id();

  let isExpanded = $state(false);

  let context: DropdownContext = {
    get isExpanded() {
      return isExpanded;
    },
    triggerId: `dropdown-trigger-${uniqueId}`,
    contentId: `dropdown-content-${uniqueId}`,
    toggle: () => (isExpanded = !isExpanded),
    close: () => (isExpanded = false),
  };

  setDropdown(context);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isExpanded) {
      isExpanded = false;
    }
  }
</script>

<div
  class={['container', className]}
  {@attach clickOutside(() => (isExpanded = false))}
  onkeydown={handleKeydown}
  {...restProps}
>
  {@render children()}
</div>

<style>
  .container {
    display: inline-block;
    position: relative;
  }
</style>
