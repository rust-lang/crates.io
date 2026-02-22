<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import { resolve } from '$app/paths';

  import * as SideMenu from '$lib/components/side-menu';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();
</script>

<div class={['page', className]} {...restProps}>
  <SideMenu.Root data-test-settings-menu>
    <SideMenu.Item href={resolve('/settings/profile')}>Profile</SideMenu.Item>
    <SideMenu.Item href={resolve('/settings/tokens')} data-test-tokens>API Tokens</SideMenu.Item>
  </SideMenu.Root>

  <div class="content">
    {@render children()}
  </div>
</div>

<style>
  .page {
    display: grid;
    gap: var(--space-s);

    @media (min-width: 768px) {
      grid-template:
        'menu content' auto /
        200px auto;
    }
  }

  .content {
    :global(h2):first-child {
      margin-top: var(--space-3xs);
    }
  }
</style>
