<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import CrateIcon from '$lib/assets/crate.svg?component';
  import DownloadIcon from '$lib/assets/download.svg?component';
  import CrateSidebar from '$lib/components/crate-sidebar/CrateSidebar.svelte';
  import CrateHeader from '$lib/components/CrateHeader.svelte';

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Keyword = components['schemas']['Keyword'];
  type Owner = components['schemas']['Owner'];

  interface Props {
    crate: Crate;
    version: Version;
    keywords?: Keyword[];
    requestedVersion?: string;
  }

  let { crate, version, keywords = [], requestedVersion }: Props = $props();

  // TODO load owners from API
  let owners: Owner[] = [];

  let numberFormat = new Intl.NumberFormat();

  let downloadsContext = $derived(requestedVersion ? version : crate);
</script>

<svelte:head>
  <title>{crate.name} - crates.io: Rust Package Registry</title>
</svelte:head>

<CrateHeader {crate} {version} versionNum={requestedVersion} {keywords} />

<div class="crate-info">
  <div class="docs" data-test-docs>
    <!-- TODO: Implement readme loading and display -->
    <!-- TODO: Implement loading spinner with Placeholder components -->
    <!-- TODO: Implement readme error state with retry button -->
    <!-- TODO: Implement no readme state -->
    README content goes here.
  </div>

  <div class="sidebar">
    <CrateSidebar {crate} {version} {owners} requestedVersion={requestedVersion !== undefined} />
  </div>
</div>

<div class="crate-downloads">
  <div class="stats">
    {#if 'num' in downloadsContext && downloadsContext.num}
      <h3 data-test-crate-stats-label>
        Stats Overview for {downloadsContext.num}
        <a href={resolve('/crates/[crate_id]', { crate_id: crate.id })}>(see all)</a>
      </h3>
    {:else}
      <h3 data-test-crate-stats-label>Stats Overview</h3>
    {/if}

    <div class="stat">
      <span class="num">
        <DownloadIcon />
        <span class="num__align">{numberFormat.format(downloadsContext.downloads)}</span>
      </span>
      <span class="text--small">Downloads all time</span>
    </div>

    <div class="stat">
      <span class="num">
        <CrateIcon />
        <span class="num__align">{crate.num_versions}</span>
      </span>
      <span class="text--small">Versions published</span>
    </div>
  </div>

  <!-- TODO: Implement download graph with stacked/unstacked toggle -->
</div>

<style>
  .crate-info {
    @media only screen and (min-width: 890px) {
      display: grid;
      grid-template-columns: minmax(0, 7fr) minmax(0, 3fr);
    }
  }

  .docs {
    --shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);

    margin-bottom: var(--space-l);
    padding: var(--space-m) var(--space-l);
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: var(--shadow);

    @media only screen and (max-width: 550px) {
      margin-left: calc(var(--main-layout-padding) * -1);
      margin-right: calc(var(--main-layout-padding) * -1);
      border-radius: 0;
    }

    @media only screen and (min-width: 890px) {
      margin-bottom: 0;
    }
  }

  .sidebar {
    @media only screen and (min-width: 890px) {
      margin-top: var(--space-m);
      margin-left: var(--space-m);
    }
  }

  .crate-downloads {
    display: flex;
    flex-wrap: wrap;
    margin-top: var(--space-l);
    border-top: 5px solid var(--gray-border);

    h3 {
      width: 100%;
    }
  }

  .stats {
    flex-grow: 7;
    display: flex;
    flex-wrap: wrap;
  }

  .stat {
    border-left: 1px solid var(--gray-border);
    padding: var(--space-s) var(--space-m);
    display: flex;
    flex-wrap: wrap;
    flex-direction: column;
    flex-grow: 1;

    .num {
      font-size: 160%;
      font-weight: bold;
      margin-bottom: var(--space-3xs);
    }

    .num__align {
      position: relative;
      bottom: 0.4rem;
    }
  }
</style>
