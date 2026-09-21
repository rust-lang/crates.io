<!--
  @component
  Renders a linked external account as a provider-labelled chip.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import Tooltip from './Tooltip.svelte';

  /** Extend this union when support for other linked-account providers is added. */
  type Provider = 'github';

  interface Props {
    /** The linked-account provider. */
    provider: Provider;

    /** The account handle displayed in the chip. */
    handle: string;

    /** The URL of the linked account. */
    href: string;

    /** The crates.io username that does not resolve to this user through GitHub lookup. */
    mismatchedUsername?: string;
  }

  let { provider, handle, href, mismatchedUsername }: Props = $props();

  let mismatchDescription = $derived(
    `This crates.io account may differ from the GitHub account named "${mismatchedUsername}".`,
  );
  const mismatchDescriptionId = $props.id();

  let handleElement = $state<HTMLSpanElement>();
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve -->
<a
  {href}
  class={['account-chip', mismatchedUsername && 'mismatched']}
  aria-describedby={mismatchedUsername ? mismatchDescriptionId : undefined}
  data-test-account-chip
>
  {#if provider === 'github'}
    <Icon class="i-simple-icons:github" label="GitHub" data-test-provider-icon />
  {/if}
  <span bind:this={handleElement} class="handle" data-test-handle>{handle}</span>
  {#if mismatchedUsername}
    <span class="mismatch-marker" aria-hidden="true" data-test-mismatch-marker>≠</span>
  {/if}
  <Tooltip onlyWhenTruncated={!mismatchedUsername} truncationTarget={handleElement}>
    <span class="tooltip-text" aria-hidden="true">
      {handle}
      {#if mismatchedUsername}
        <br />
        {mismatchDescription}
      {/if}
    </span>
  </Tooltip>
</a>

{#if mismatchedUsername}
  <span id={mismatchDescriptionId} hidden data-test-mismatch-description>{mismatchDescription}</span>
{/if}

<!-- eslint-enable svelte/no-navigation-without-resolve -->

<style>
  .account-chip {
    --icon-size: 1.125em;

    display: inline-flex;
    align-items: center;
    gap: 0.5em;
    max-width: 100%;
    min-width: 0;
    border: 1px solid var(--gray-border);
    border-radius: var(--border-radius-pill);
    padding: 0.375em 0.625em;
    font-size: var(--space-xs);
    color: var(--main-color);

    &:hover {
      background: light-dark(white, #232321);
    }
  }

  .mismatched {
    border-color: light-dark(var(--orange-700), var(--orange-300));
  }

  .handle {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mismatch-marker {
    flex-shrink: 0;
    color: light-dark(var(--orange-700), var(--orange-300));
    font-family: var(--font-monospace);
    font-weight: 700;
  }

  .tooltip-text {
    display: block;
    overflow-wrap: anywhere;
  }
</style>
