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

  let isActive = $derived(active ?? page.url.pathname === href);
</script>

<li {...restProps}>
  <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
  <a {href} class="link" class:active={isActive} data-test-active={isActive}>
    {@render children()}
  </a>
</li>

<style>
  .link {
    display: block;
    color: var(--main-color);
    transition:
      color var(--transition-medium),
      border-bottom-color var(--transition-medium);

    &.active {
      color: var(--link-hover-color);
      background: var(--main-bg-dark);
    }

    &:hover {
      color: var(--link-hover-color);
      transition:
        color var(--transition-instant),
        border-bottom-color var(--transition-instant);
    }

    &:focus-visible {
      outline: none;
      margin: -3px;
      border: 3px solid var(--yellow500);
      position: relative;
      transition: border-bottom-color var(--transition-instant);
      z-index: 1;
    }

    @media only screen and (min-width: 551px) {
      padding: calc(var(--nav-tabs-padding-v) + var(--nav-tabs-border-width)) var(--nav-tabs-padding-h)
        var(--nav-tabs-padding-v);
      border-top-left-radius: var(--nav-tabs-radius);
      border-top-right-radius: var(--nav-tabs-radius);
      border-bottom: var(--nav-tabs-border-width) solid transparent;
      margin-bottom: calc(0px - var(--nav-tabs-border-width));

      &.active,
      &:hover {
        border-bottom-color: var(--link-hover-color);
      }
    }

    @media only screen and (max-width: 550px) {
      padding: var(--nav-tabs-padding-v) var(--nav-tabs-padding-h) var(--nav-tabs-padding-v)
        calc(var(--nav-tabs-padding-h) + var(--nav-tabs-border-width));
      border-top-right-radius: var(--nav-tabs-radius);
      border-bottom-right-radius: var(--nav-tabs-radius);
      border-left: var(--nav-tabs-border-width) solid transparent;
      margin-left: calc(0px - var(--nav-tabs-border-width));

      &.active,
      &:hover {
        border-left-color: var(--link-hover-color);
      }
    }
  }
</style>
