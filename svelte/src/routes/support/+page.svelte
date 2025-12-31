<script lang="ts">
  import { page } from '$app/state';

  import PageHeader from '$lib/components/PageHeader.svelte';
  import CrateReportForm from '$lib/components/support/CrateReportForm.svelte';
  import TextContent from '$lib/components/TextContent.svelte';

  const SUPPORTS = [
    {
      inquire: 'crate-violation',
      label: 'Report a crate that violates policies',
    },
  ];

  let inquire = $derived(page.url.searchParams.get('inquire'));
  let crate = $derived(page.url.searchParams.get('crate') || undefined);
</script>

<PageHeader title="Contact Us" />

<TextContent data-test-id="support-main-content">
  {#if inquire === 'crate-violation'}
    <section data-test-id="crate-violation-section">
      <CrateReportForm {crate} />
    </section>
  {:else}
    <section data-test-id="inquire-list-section">
      <h2>Choose one of the these categories to continue.</h2>
      <ul class="inquire-list" data-test-id="inquire-list">
        {#each SUPPORTS as support (support.inquire)}
          <li>
            <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolve() doesn't support query params -->
            <a href="?inquire={support.inquire}" data-test-id="link-{support.inquire}" class="link box-link">
              {support.label}
            </a>
          </li>
        {/each}
        <li>
          <a href="mailto:help@crates.io" data-test-id="link-email-support" class="link box-link">
            For all other cases:
            <strong>help@crates.io</strong>
          </a>
        </li>
      </ul>
    </section>
  {/if}
</TextContent>

<style>
  .inquire-list {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-s);
    list-style: none;
    padding: 0;
  }

  .link {
    --shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);

    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: inherit;
    min-height: var(--space-2xl);
    padding: var(--space-xs) var(--space-s);
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

    strong {
      margin-left: var(--space-3xs);
      font-weight: 500;
    }
  }
</style>
