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

  // `scrollIntoView()` would also scroll the page, for example away from a
  // README anchor. Once Firefox and Safari support the `container` option,
  // this can be replaced with:
  //
  //   link.scrollIntoView({ container: 'nearest', inline: 'nearest', block: 'nearest' });
  function scrollIntoListView(link: HTMLElement) {
    let list = link.closest('ul');
    if (!list) return;

    let listRect = list.getBoundingClientRect();
    let linkRect = link.getBoundingClientRect();
    if (linkRect.left < listRect.left || linkRect.right > listRect.right) {
      list.scrollLeft += linkRect.left + linkRect.width / 2 - (listRect.left + listRect.width / 2);
    }
  }
</script>

<li {...restProps}>
  <!-- eslint-disable svelte/no-navigation-without-resolve -->
  <a
    {href}
    class="link"
    class:active={isActive}
    data-test-active={isActive || undefined}
    {@attach isActive && scrollIntoListView}
  >
    {@render children()}
  </a>
  <!-- eslint-enable svelte/no-navigation-without-resolve -->
</li>

<style>
  .link {
    display: block;
    padding: calc(var(--nav-tabs-padding-v) + var(--nav-tabs-border-width)) var(--nav-tabs-padding-h)
      var(--nav-tabs-padding-v);
    border-top-left-radius: var(--nav-tabs-radius);
    border-top-right-radius: var(--nav-tabs-radius);
    border-bottom: var(--nav-tabs-border-width) solid transparent;
    white-space: nowrap;
    color: var(--main-color);
    transition:
      color var(--transition-medium),
      border-bottom-color var(--transition-medium);

    &.active {
      color: var(--link-hover-color);
      background: var(--main-bg-dark);
    }

    &.active,
    &:hover {
      border-bottom-color: var(--link-hover-color);
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
      border: 3px solid var(--focus-ring-color);
      position: relative;
      transition: border-bottom-color var(--transition-instant);
      z-index: 1;
    }
  }
</style>
