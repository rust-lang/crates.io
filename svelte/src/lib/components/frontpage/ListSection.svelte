<script lang="ts" generics="T extends { id: string }">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import ListItemPlaceholder from './ListItemPlaceholder.svelte';

  interface Props extends HTMLAttributes<HTMLElement> {
    title: string;
    href: string;
    items: T[] | undefined;
    withSubtitle?: boolean;
    ordered?: boolean;
    item: Snippet<[item: T, index: number]>;
  }

  let { title, href, items, withSubtitle = false, ordered = true, item, ...restProps }: Props = $props();
</script>

<section {...restProps}>
  <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
  <h2><a {href}>{title}</a></h2>
  <svelte:element this={ordered ? 'ol' : 'ul'} class="list" aria-busy={!items}>
    {#if !items}
      {#each { length: 10 } as _, i (i)}
        <li><ListItemPlaceholder {withSubtitle} /></li>
      {/each}
    {:else}
      {#each items as it, index (it.id)}
        <li>{@render item(it, index)}</li>
      {/each}
    {/if}
  </svelte:element>
</section>

<style>
  h2 {
    font-size: 1.05rem;

    a:not(:hover) {
      color: var(--main-color);
    }
  }

  .list {
    list-style: none;
    padding: 0;

    > * + * {
      margin-top: var(--space-2xs);
    }
  }
</style>
