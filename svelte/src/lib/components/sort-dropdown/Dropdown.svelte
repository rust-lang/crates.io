<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import SortIcon from '$lib/assets/sort.svg?component';
  import * as Dropdown from '$lib/components/dropdown';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    current: string;
    children: Snippet;
  }

  let { current, children, class: className, ...restProps }: Props = $props();
</script>

<div class={['sort-dropdown', className]} {...restProps}>
  <Dropdown.Root>
    <Dropdown.Trigger class="trigger" data-test-current-order>
      <SortIcon class="icon" />
      {current}
    </Dropdown.Trigger>

    <Dropdown.Menu>
      {@render children()}
    </Dropdown.Menu>
  </Dropdown.Root>
</div>

<style>
  .sort-dropdown {
    display: inline-block;

    & :global(.trigger) {
      background-color: var(--main-bg-dark);
      font-size: 85%;
      padding: var(--space-2xs);
      border: none;
      border-radius: var(--space-3xs);
    }

    & :global(.icon) {
      color: #1a9c5d;
      margin-right: var(--space-2xs);
    }
  }
</style>
