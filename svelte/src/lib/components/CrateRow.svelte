<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { HTMLAttributes } from 'svelte/elements';

  import { resolve } from '$app/paths';
  import { formatDistanceToNow, formatISO } from 'date-fns';

  import CopyIcon from '$lib/assets/copy.svg?component';
  import DownloadIcon from '$lib/assets/download.svg?component';
  import LatestUpdatesIcon from '$lib/assets/latest-updates.svg?component';
  import CopyButton from '$lib/components/CopyButton.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { truncateText } from '$lib/utils/truncate-text';

  type Crate = components['schemas']['Crate'];

  interface Props extends HTMLAttributes<HTMLDivElement> {
    crate: Crate;
  }

  let { crate, ...restProps }: Props = $props();

  let showVersion = $derived(crate.default_version && !crate.yanked);
  let cargoTomlSnippet = $derived(`${crate.name} = "${crate.default_version}"`);
</script>

<div class="crate-row" data-test-crate-row {...restProps}>
  <div class="description-box">
    <div class="crate-spec" role="heading" aria-level={2} data-test-crate-spec>
      <a href={resolve('/crates/[crate_id]', { crate_id: crate.id })} class="name" data-test-crate-link>
        {crate.name}
      </a>
      {#if showVersion}
        <span class="version" data-test-version>v{crate.default_version}</span>
        <CopyButton
          copyText={cargoTomlSnippet}
          title="Copy Cargo.toml snippet to clipboard"
          class="copy-button button-reset"
          data-test-copy-toml-button
        >
          <CopyIcon aria-label="Copy Cargo.toml snippet to clipboard" />
        </CopyButton>
      {/if}
    </div>
    {#if crate.description}
      <div class="description text--small" data-test-description>
        {truncateText(crate.description)}
      </div>
    {/if}
  </div>

  <div class="stats">
    <div class="downloads" data-test-downloads>
      <DownloadIcon class="download-icon" />
      <span>
        <span>
          All-Time:
          <Tooltip text="Total number of downloads" />
        </span>
        {crate.downloads.toLocaleString()}
      </span>
    </div>
    <div class="recent-downloads" data-test-recent-downloads>
      <DownloadIcon class="download-icon" />
      <span>
        <span>
          Recent:
          <Tooltip text="Downloads in the last 90 days" />
        </span>
        {(crate.recent_downloads ?? 0).toLocaleString()}
      </span>
    </div>
    <div class="updated-at">
      <LatestUpdatesIcon height="32" width="32" />
      <span>
        <span>
          Updated:
          <Tooltip text="The last time the crate was updated" />
        </span>
        <time datetime={formatISO(crate.updated_at)} data-test-updated-at>
          {formatDistanceToNow(crate.updated_at, { addSuffix: true })}
          <Tooltip text={crate.updated_at} />
        </time>
      </span>
    </div>
  </div>

  {#if crate.homepage || crate.documentation || crate.repository}
    <ul class="quick-links">
      {#if crate.homepage}
        <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
        <li><a href={crate.homepage}>Homepage</a></li>
      {/if}
      {#if crate.documentation}
        <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
        <li><a href={crate.documentation}>Documentation</a></li>
      {/if}
      {#if crate.repository}
        <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
        <li><a href={crate.repository}>Repository</a></li>
      {/if}
    </ul>
  {/if}
</div>

<style>
  .crate-row {
    --shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);

    display: flex;
    flex-wrap: wrap;
    padding: var(--space-s-m) var(--space-m-l);
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: var(--shadow);
  }

  .description-box {
    display: flex;
    flex-direction: column;
    width: 70%;
  }

  .name {
    color: var(--main-color);
    font-weight: bold;
    text-decoration: none;
    font-size: 120%;
    overflow-wrap: break-word;
  }

  .crate-row :global(.copy-button) {
    color: var(--main-color);
    cursor: pointer;

    /* Hover selector for pointer only */
    /* See: https://github.com/rust-lang/crates.io/issues/10772 */
    @media (pointer: fine) {
      opacity: 0;
      transition: var(--transition-medium);
    }

    :global(svg) {
      vertical-align: top;
      height: 1rem;
      width: 1rem;
    }
  }

  @media (pointer: fine) {
    .crate-row {
      &:hover :global(.copy-button) {
        opacity: 0.8;
        transition: var(--transition-instant);
      }

      &:hover :global(.copy-button:hover),
      :global(.copy-button:focus) {
        opacity: 1;
        transition: var(--transition-instant);
      }
    }
  }

  .crate-spec {
    display: flex;
    flex-wrap: wrap;
    align-items: center;

    & > :global(*) {
      margin-bottom: calc(var(--space-xs) / 2);
    }

    & > :global(:not(:last-child)) {
      margin-right: var(--space-2xs);
    }
  }

  .description {
    margin-top: calc(var(--space-xs) / 2);
    line-height: 1.5;
  }

  .stats {
    width: 30%;
    color: var(--main-color-light);

    > :global(* + *) {
      margin-top: var(--space-xs);
    }
  }

  .stats :global(svg) {
    height: 1em;
    width: 1em;
    margin-right: var(--space-xs);
    flex-shrink: 0;
  }

  .stats :global(svg.download-icon) {
    height: calc(1em + 20px);
    width: calc(1em + 20px);
    margin: -10px;
    margin-right: calc(var(--space-xs) - 10px);
  }

  .stats :global(svg.download-icon circle) {
    fill: none;
  }

  .downloads {
    display: flex;
    align-items: center;
  }

  .recent-downloads {
    display: flex;
    align-items: center;
  }

  .updated-at {
    display: flex;
    align-items: center;
  }

  ul.quick-links {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    font-size: 80%;
    list-style-type: none;
    margin: var(--space-xs) 0 0 0;
    padding: 0;

    > :global(* + *) {
      margin-left: var(--space-xs);
    }
  }
</style>
