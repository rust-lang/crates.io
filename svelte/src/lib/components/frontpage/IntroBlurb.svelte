<script lang="ts">
  import type { operations } from '@crates-io/api-client';

  import StatsValue from './StatsValue.svelte';

  type Summary = operations['get_summary']['responses']['200']['content']['application/json'];

  interface Props {
    summary?: Summary;
  }

  let { summary }: Props = $props();

  const numberFormat = new Intl.NumberFormat();
</script>

<div class="blurb">
  <div class="intro">
    Instantly publish your crates and install them. Use the API to interact and find out more information about
    available crates. Become a contributor and enhance the site with your work.
  </div>

  <div class="stats">
    <StatsValue
      value={summary ? numberFormat.format(summary.num_downloads) : '---,---,---'}
      label="Downloads"
      iconClass="i-mdi:cloud-download"
      data-test-total-downloads
    />
    <StatsValue
      value={summary ? numberFormat.format(summary.num_crates) : '---,---'}
      label="Crates in stock"
      iconClass="i-mdi:package-variant-closed"
      data-test-total-crates
    />
  </div>
</div>

<style>
  .blurb {
    margin: var(--space-l) var(--space-s);
    display: flex;
    gap: var(--space-l);

    @media only screen and (max-width: 650px) {
      flex-direction: column;
      align-items: center;
    }
  }

  .intro {
    flex: 6;
    line-height: 1.5;
  }

  .stats {
    flex: 4;
    display: flex;
    flex-direction: column;

    > :global(* + *) {
      margin-top: var(--space-s);
    }
  }
</style>
