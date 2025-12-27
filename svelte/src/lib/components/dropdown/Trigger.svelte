<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  import { getDropdown } from './context';

  interface Props extends HTMLButtonAttributes {
    children: Snippet;
    hideArrow?: boolean;
  }

  let { children, hideArrow = false, class: className, ...restProps }: Props = $props();

  let dropdown = getDropdown();
</script>

<button
  type="button"
  id={dropdown.triggerId}
  aria-expanded={dropdown.isExpanded}
  aria-controls={dropdown.contentId}
  onclick={dropdown.toggle}
  class={['trigger', className]}
  class:active={dropdown.isExpanded}
  {...restProps}
>
  {@render children()}
  {#if !hideArrow}
    <span class="arrow"></span>
  {/if}
</button>

<style>
  .trigger {
    display: inline-flex;
    align-items: center;
    color: inherit;
    cursor: pointer;
  }

  .arrow {
    margin-left: var(--space-2xs);
    font-size: 50%;
    display: inline-block;
    vertical-align: middle;
  }

  .arrow::after {
    content: '\25BC'; /* ▼ */
  }

  .active .arrow::after {
    content: '\25B2'; /* ▲ */
  }
</style>
