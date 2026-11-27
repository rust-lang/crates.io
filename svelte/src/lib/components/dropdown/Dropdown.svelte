<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import type { DropdownContext } from './context';

  import { setDropdown } from './context';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();
  let uniqueId = $props.id();

  let isExpanded = $state(false);
  let trigger = $state<HTMLButtonElement>();

  let context: DropdownContext = {
    get isExpanded() {
      return isExpanded;
    },
    set isExpanded(value) {
      isExpanded = value;
    },
    get trigger() {
      return trigger;
    },
    set trigger(element) {
      trigger = element;
    },
    triggerId: `dropdown-trigger-${uniqueId}`,
    contentId: `dropdown-content-${uniqueId}`,
    toggle: () => (isExpanded = !isExpanded),
    close: () => (isExpanded = false),
  };

  setDropdown(context);
</script>

<div class={['container', className]} {...restProps}>
  {@render children()}
</div>

<style>
  .container {
    display: inline-flex;
    align-items: center;
  }
</style>
