<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import { page } from '$app/state';

  interface Props extends HTMLAttributes<HTMLLIElement> {
    href: string;
    active?: boolean;
    children: Snippet;
  }

  let { href, active, children, ...restProps }: Props = $props();

  let isActive = $derived(active ?? (page.url.pathname === href || page.url.pathname.startsWith(href + '/')));
</script>

<li {...restProps}>
  <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
  <a {href} class="link" class:active={isActive}>
    {@render children()}
  </a>
</li>

<style>
  .link {
    display: block;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--space-3xs);
    color: var(--main-color-light);
    transition: all var(--transition-medium) ease-in;

    &:hover {
      background-color: var(--main-bg-dark);
      color: var(--main-color);
      transition: none;
    }
  }

  .active {
    background-color: var(--main-bg-dark);
    color: var(--main-color);

    &:hover {
      background-color: light-dark(#e5e1cd, #262522);
    }
  }
</style>
