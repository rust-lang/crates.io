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

    /** Whether the account handle differs from the crates.io username. */
    mismatched?: boolean;
  }

  let { provider, handle, href, mismatched = false }: Props = $props();

  let handleElement = $state<HTMLSpanElement>();
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve -->
<a {href} class={['account-chip', mismatched && 'mismatched']} data-test-account-chip>
  {#if provider === 'github'}
    <Icon class="i-simple-icons:github" label="GitHub" data-test-provider-icon />
  {/if}
  <span bind:this={handleElement} class="handle" data-test-handle>{handle}</span>
  {#if mismatched}
    <span class="mismatch-marker" aria-hidden="true" data-test-mismatch-marker>≠</span>
    <span class="sr-only" data-test-mismatch-description> does not match the crates.io username</span>
  {/if}
  <Tooltip onlyWhenTruncated={!mismatched} truncationTarget={handleElement}>
    <span class="tooltip-text">{mismatched ? `${handle} does not match the crates.io username` : handle}</span>
  </Tooltip>
</a>

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
    border-radius: 99999px;
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
