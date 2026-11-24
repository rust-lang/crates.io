<script module lang="ts">
  import type { Placement, Side } from '@floating-ui/dom';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Popover from './Popover.svelte';

  const SIDES: Side[] = ['top', 'right', 'bottom', 'left'];
  const PLACEMENTS = SIDES.flatMap((side): Placement[] => [side, `${side}-start`, `${side}-end`]);

  const { Story } = defineMeta({
    title: 'Popover',
    component: Popover,
    tags: ['autodocs'],
    argTypes: {
      placement: { control: 'select', options: PLACEMENTS },
      open: { table: { disable: true } },
      anchor: { table: { disable: true } },
      children: { table: { disable: true } },
    },
    args: {
      placement: 'bottom-start',
      minWidthFromAnchor: false,
    },
    parameters: {
      docs: { story: { height: '250px' } },
    },
  });
</script>

<script lang="ts">
  let open = $state(false);
  let anchor = $state<HTMLButtonElement>();
</script>

<Story name="Default" parameters={{ chromatic: { disableSnapshot: true } }}>
  {#snippet template(args)}
    <div class="container">
      <button type="button" bind:this={anchor}>Toggle popover</button>

      <Popover bind:open {anchor} placement={args.placement} minWidthFromAnchor={args.minWidthFromAnchor}>
        <p class="content">Content</p>
      </Popover>
    </div>
  {/snippet}
</Story>

<style>
  .container {
    display: grid;
    place-items: center;
    height: 200px;
  }

  .content {
    margin: 0;
    padding: var(--space-xs);
    background: var(--surface);
    border: 1px solid var(--gray-border);
    border-radius: var(--border-radius-small);
  }
</style>
