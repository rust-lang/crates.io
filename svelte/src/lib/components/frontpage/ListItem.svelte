<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import ChevronRightIcon from '$lib/assets/chevron-right.svg?component';

  interface Props extends HTMLAttributes<HTMLAnchorElement> {
    title: string;
    subtitle?: string;
    href: string;
  }

  let { title, subtitle, href, class: className, ...restProps }: Props = $props();
</script>

<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
<a {href} class={['box', className]} {...restProps}>
  <div class="left">
    <div class="title">{title}</div>
    {#if subtitle}<div class="subtitle">{subtitle}</div>{/if}
  </div>
  <ChevronRightIcon class="right" />
</a>

<style>
  .box {
    --shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);

    display: flex;
    align-items: center;
    width: 100%;
    height: var(--space-2xl);
    padding: 0 var(--space-s);
    background-color: light-dark(white, #141413);
    color: light-dark(#525252, #f9f7ec);
    text-decoration: none;
    border-radius: var(--space-3xs);
    box-shadow: var(--shadow);
    transition: background-color var(--transition-slow);

    &:focus-visible {
      outline: none;
      box-shadow:
        0 0 0 3px var(--yellow500),
        var(--shadow);
    }

    &:hover,
    &:focus-visible {
      color: light-dark(#525252, #f9f7ec);
      background-color: light-dark(hsl(58deg 72% 97%), hsl(204, 3%, 11%));
      transition: background-color var(--transition-instant);
    }

    &:active {
      transform: translateY(2px);
      --shadow: inset 0 0 0 1px hsla(51, 50%, 44%, 0.15);
    }
  }

  .left {
    flex-grow: 1;
    width: 0;
  }

  .title,
  .subtitle {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: 16px;
  }

  .subtitle {
    margin-top: var(--space-3xs);
    font-size: 13px;
    color: light-dark(rgb(118, 131, 138), #cccac2);
  }

  .box :global(.right) {
    flex-shrink: 0;
    height: var(--space-s);
    width: auto;
    margin-left: var(--space-xs);
    color: light-dark(rgb(118, 131, 138), #cccac2);
  }
</style>
