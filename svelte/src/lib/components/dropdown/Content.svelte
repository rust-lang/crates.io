<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import { getDropdown } from './context';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();

  let dropdown = getDropdown();
</script>

{#if dropdown.isExpanded}
  <div id={dropdown.contentId} class={['dropdown', className]} {...restProps}>
    {@render children()}
  </div>
{/if}

<style>
  .dropdown {
    position: absolute;
    width: 100%;
    z-index: 10;
  }
</style>
