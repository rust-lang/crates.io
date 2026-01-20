<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { DownloadChartData } from '$lib/components/download-chart/data';

  import { resolve } from '$app/paths';

  import CrateIcon from '$lib/assets/crate.svg?component';
  import DownloadIcon from '$lib/assets/download.svg?component';
  import CrateSidebar from '$lib/components/crate-sidebar/CrateSidebar.svelte';
  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import DownloadChart from '$lib/components/download-chart/DownloadChart.svelte';
  import * as Dropdown from '$lib/components/dropdown';
  import ReadmePlaceholder from '$lib/components/ReadmePlaceholder.svelte';
  import RenderedHtml from '$lib/components/RenderedHtml.svelte';
  import { loadReadme } from '$lib/utils/readme';

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Keyword = components['schemas']['Keyword'];
  type Owner = components['schemas']['Owner'];

  interface Props {
    crate: Crate;
    version: Version;
    keywords?: Keyword[];
    owners: Owner[];
    requestedVersion?: string;
    readmePromise: Promise<string | null>;
    downloadsPromise: Promise<DownloadChartData>;
  }

  let { crate, version, keywords = [], owners, requestedVersion, readmePromise, downloadsPromise }: Props = $props();

  let numberFormat = new Intl.NumberFormat();
  let stackedGraph = $state(true);

  let downloadsContext = $derived(requestedVersion ? version : crate);

  let retryReadmePromise = $state<typeof readmePromise | null>(null);
  let activeReadmePromise = $derived(retryReadmePromise ?? readmePromise);

  function retryReadme() {
    retryReadmePromise = loadReadme(fetch, crate.name, version.num);
  }
</script>

<svelte:head>
  <title>{crate.name} - crates.io: Rust Package Registry</title>
</svelte:head>

<CrateHeader {crate} {version} versionNum={requestedVersion} {keywords} />

<div class="crate-info">
  <div class="docs" data-test-docs>
    {#await activeReadmePromise}
      <ReadmePlaceholder />
    {:then readme}
      {#if readme}
        <article aria-label="Readme" data-test-readme>
          <RenderedHtml html={readme} />
        </article>
      {:else}
        <div class="no-readme" data-test-no-readme>
          {crate.name} v{version.num} appears to have no <code>README.md</code> file
        </div>
      {/if}
    {:catch}
      <div class="readme-error" data-test-readme-error>
        Failed to load <code>README</code> file for {crate.name} v{version.num}
        <button type="button" class="retry-button button" data-test-retry-button onclick={retryReadme}>Retry</button>
      </div>
    {/await}
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

  <div class="graph">
    <h4>Downloads over the last 90 days</h4>
    <div class="toggle-stacked">
      <span class="toggle-stacked-label">Display as </span>
      <Dropdown.Root>
        <Dropdown.Trigger class="trigger">
          <span class="trigger-label">
            {stackedGraph ? 'Stacked' : 'Unstacked'}
          </span>
        </Dropdown.Trigger>
        <Dropdown.Menu>
          <Dropdown.Item>
            <button type="button" class="dropdown-button" onclick={() => (stackedGraph = true)}>Stacked</button>
          </Dropdown.Item>
          <Dropdown.Item>
            <button type="button" class="dropdown-button" onclick={() => (stackedGraph = false)}>Unstacked</button>
          </Dropdown.Item>
        </Dropdown.Menu>
      </Dropdown.Root>
    </div>
    {#await downloadsPromise then downloads}
      <div class="graph-data">
        <DownloadChart data={downloads} stacked={stackedGraph} />
      </div>
    {:catch}
      <div class="graph-error">Failed to load download data</div>
    {/await}
  </div>
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

  .no-readme,
  .readme-error {
    padding: var(--space-l) var(--space-s);
    text-align: center;
    font-size: 20px;
    font-weight: 300;
    overflow-wrap: break-word;
    line-height: 1.5;

    code {
      font-size: 18px;
      font-weight: 500;
    }
  }

  .retry-button {
    display: block;
    text-align: center;
    margin: var(--space-s) auto 0;
  }

  .graph {
    flex-grow: 10;
    width: 100%;
    margin: var(--space-xs) 0 var(--space-m);

    h4 {
      color: var(--main-color-light);
      float: left;
    }

    @media only percy {
      display: none;
    }
  }

  .graph-data {
    clear: both;
  }

  .graph-error {
    clear: both;
    padding: var(--space-l) var(--space-s);
    text-align: center;
    font-size: 20px;
    font-weight: 300;
  }

  .toggle-stacked {
    float: right;
    margin-top: calc(1.33em - 10px);
    margin-bottom: calc(1.33em - 10px);

    :global(.trigger) {
      background-color: var(--main-bg-dark);
      font-size: 85%;
      padding: 10px;
      border: none;
      border-radius: 5px;
    }

    .trigger-label {
      min-width: 65px;
    }

    .dropdown-button {
      background: none;
      border: 0;
    }
  }
</style>
