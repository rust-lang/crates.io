<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import type { DropdownContext } from './context';

  import { clickOutside } from './clickOutside';
  import { setDropdown } from './context';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();

  let isExpanded = $state(false);
  let triggerId = `dropdown-trigger-${crypto.randomUUID().slice(0, 8)}`;
  let contentId = `dropdown-content-${crypto.randomUUID().slice(0, 8)}`;

  let context: DropdownContext = {
    get isExpanded() {
      return isExpanded;
    },
    triggerId,
    contentId,
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
  use:clickOutside={() => (isExpanded = false)}
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
