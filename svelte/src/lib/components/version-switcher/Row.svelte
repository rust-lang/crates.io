<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  type Version = components['schemas']['Version'];

  /** The fields of a version that a row shows. */
  export type VersionRowData = Pick<Version, 'num' | 'created_at' | 'yanked'>;
</script>

<script lang="ts">
  import type { ResolvedPathname } from '$app/types';

  interface Props {
    version: VersionRowData;
    href: ResolvedPathname;
    isCurrent?: boolean;
    isDefault?: boolean;
    onClick?: () => void;
  }

  let { version, href, isCurrent = false, isDefault = false, onClick }: Props = $props();

  const DATE_FORMAT = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });

  let date = $derived(DATE_FORMAT.format(new Date(version.created_at)));

  let status = $derived(version.yanked ? 'yanked' : `published ${date}`);
  let label = $derived([version.num, isDefault && 'default version', status].filter(Boolean).join(', '));
</script>

<a
  {href}
  onclick={onClick}
  class="row"
  class:current={isCurrent}
  class:yanked={version.yanked}
  aria-current={isCurrent ? 'page' : undefined}
  aria-label={label}
  data-test-row
>
  <span class="num">{version.num}</span>
  {#if isDefault}
    <span class="default-label" data-test-default>Default</span>
  {/if}
  {#if version.yanked}
    <span class="meta">yanked</span>
  {:else}
    <time class="meta" datetime={version.created_at} data-test-date>{date}</time>
  {/if}
</a>

<style>
  .row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2xs);
    padding: var(--space-3xs) var(--space-xs);
    color: inherit;
    font-variant-numeric: tabular-nums;
    text-decoration: none;

    &:hover {
      background: light-dark(var(--main-bg), hsl(0, 1%, 13%));
    }

    @media (max-width: 550px) {
      padding-block: var(--space-xs);
    }
  }

  /* The panel is darker than the page in dark mode, so the active color of tabs and the side menu is inverted there. */
  .current {
    background: light-dark(var(--main-bg-dark), var(--main-bg));

    &:hover {
      background: light-dark(#e5e1cd, hsl(0, 1%, 24%));
    }
  }

  .num {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .default-label {
    flex-shrink: 0;
    align-self: center;
    padding: 0 var(--space-3xs);
    border: 1px solid var(--gray-border);
    border-radius: var(--border-radius-pill);
    font-size: 11px;
    line-height: 16px;
  }

  .meta {
    flex-shrink: 0;
    margin-left: auto;
    color: var(--main-color-light);
    font-size: 12px;
  }

  .yanked .num {
    text-decoration: line-through;
    opacity: 0.7;
  }
</style>
