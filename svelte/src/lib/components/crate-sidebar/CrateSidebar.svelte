<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { PlaygroundCrate } from '$lib/utils/playground';

  import { resolve } from '$app/paths';
  import { format, formatDistanceToNow, formatISO } from 'date-fns';
  import prettyBytes from 'pretty-bytes';
  import { MediaQuery } from 'svelte/reactivity';

  import CalendarIcon from '$lib/assets/calendar.svg?component';
  import CircleQuestionIcon from '$lib/assets/circle-question.svg?component';
  import CodeIcon from '$lib/assets/code.svg?component';
  import LicenseIcon from '$lib/assets/license.svg?component';
  import LinkIcon from '$lib/assets/link.svg?component';
  import RustIcon from '$lib/assets/rust.svg?component';
  import WeightIcon from '$lib/assets/weight.svg?component';
  import CopyButton from '$lib/components/CopyButton.svelte';
  import LicenseExpression from '$lib/components/LicenseExpression.svelte';
  import OwnersList from '$lib/components/OwnersList.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { formatShortNum } from '$lib/utils/format-short-num';
  import { buildPlaygroundLink } from '$lib/utils/playground';
  import { getPurl } from '$lib/utils/purl';
  import Edition from './Edition.svelte';
  import InstallInstructions from './InstallInstructions.svelte';
  import Link, { simplifyUrl } from './Link.svelte';
  import Msrv from './Msrv.svelte';

  const PLAYGROUND_TOOLTIP =
    'The top 100 crates are available on the Rust Playground for you to try out directly in your browser.';

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Category = components['schemas']['Category'];
  type Owner = components['schemas']['Owner'];

  interface Props {
    crate: Crate;
    version: Version;
    categories: Category[];
    owners: Owner[];
    requestedVersion?: boolean;
    playgroundCratesPromise: Promise<PlaygroundCrate[]>;
  }

  let { crate, version, categories, owners, requestedVersion = false, playgroundCratesPromise }: Props = $props();

  let canHover = new MediaQuery('hover: hover', false);

  let showHomepage = $derived.by(() => {
    let { repository, homepage } = crate;
    return homepage && (!repository || simplifyUrl(repository) !== simplifyUrl(homepage));
  });

  let hasLinks = $derived(crate.homepage || crate.repository);

  let reportUrl = $derived(`${resolve('/support')}?inquire=crate-violation&crate=${encodeURIComponent(crate.name)}`);

  let purl = $derived(getPurl(crate.name, version.num));
</script>

<section aria-label="Crate metadata" class="sidebar">
  <div class="metadata">
    <h2 class="heading">Metadata</h2>

    <time datetime={formatISO(version.created_at)} class="date">
      <CalendarIcon />
      <span>
        {formatDistanceToNow(version.created_at, { addSuffix: true })}
        <Tooltip text={format(version.created_at, 'PPP')} />
      </span>
    </time>

    {#if version.rust_version}
      <div class="msrv" data-test-msrv>
        <RustIcon />
        <Msrv msrv={version.rust_version} edition={version.edition ?? undefined} />
      </div>
    {:else if version.edition}
      <div class="edition" data-test-edition>
        <RustIcon />
        <Edition edition={version.edition} />
      </div>
    {/if}

    {#if version.license}
      <div class="license" data-test-license>
        <LicenseIcon />
        <span>
          <LicenseExpression license={version.license} />
        </span>
      </div>
    {/if}

    {#if version.linecounts?.total_code_lines}
      <div class="linecount" data-test-linecounts>
        <CodeIcon />
        <span>
          {formatShortNum(version.linecounts.total_code_lines)} SLoC
          <Tooltip>
            Source Lines of Code<br />
            <small>(excluding comments, integration tests and example code)</small>
          </Tooltip>
        </span>
      </div>
    {/if}

    {#if version.crate_size}
      <div class="bytes">
        <WeightIcon />
        {prettyBytes(version.crate_size, { binary: true })}
      </div>
    {/if}

    <div class="purl" data-test-purl>
      <LinkIcon />
      <CopyButton copyText={purl} class="button-reset purl-copy-button">
        <span class="purl-text">{purl}</span>
        <Tooltip>
          <span class="purl-tooltip">
            <strong>Package URL:</strong>
            {purl}
            <small>(click to copy)</small>
          </span>
        </Tooltip>
      </CopyButton>
      <a
        href="https://github.com/package-url/purl-spec"
        target="_blank"
        rel="noopener noreferrer"
        class="purl-help-link"
        aria-label="Learn more"
      >
        <CircleQuestionIcon />
        <Tooltip text="Learn more about Package URLs" />
      </a>
    </div>
  </div>

  {#if !version.yanked}
    <div data-test-install>
      <h2 class="heading">Install</h2>

      <InstallInstructions
        crate={crate.name}
        version={version.num}
        exactVersion={requestedVersion}
        hasLib={version.has_lib !== false}
        binNames={version.bin_names?.filter((name): name is string => Boolean(name))}
      />
    </div>
  {/if}

  {#if hasLinks}
    <div class="links">
      {#if showHomepage}
        <Link title="Homepage" url={crate.homepage!} data-test-homepage-link />
      {/if}

      <!-- TODO: Documentation link
           Requires async docs.rs status check to determine if docs are available.
           Falls back to crate.documentation if not a docs.rs link.
           See app/models/version.js documentationLink getter.
      -->

      <!-- TODO: Browse source link
           Requires async docs.rs status check.
           See app/models/version.js sourceLink getter.
      -->

      {#if crate.repository}
        <Link title="Repository" url={crate.repository} data-test-repository-link />
      {/if}
    </div>
  {/if}

  <div>
    <h2 class="heading">Owners</h2>
    <OwnersList {owners} />
  </div>

  {#if categories.length > 0}
    <div>
      <h2 class="heading">Categories</h2>
      <ul class="categories">
        {#each categories as category (category.id)}
          <li><a href={resolve('/categories/[category_id]', { category_id: category.id })}>{category.category}</a></li>
        {/each}
      </ul>
    </div>
  {/if}

  <div>
    {#await playgroundCratesPromise then playgroundCrates}
      {@const playgroundCrate = playgroundCrates.find(it => it.name === crate.name)}
      {#if playgroundCrate}
        <!-- eslint-disable svelte/no-navigation-without-resolve -->
        <a
          href={buildPlaygroundLink(playgroundCrate.id)}
          target="_blank"
          rel="noopener noreferrer"
          class="playground-button button button--small"
          data-test-playground-button
        >
          Try on Rust Playground
          {#if canHover.current}
            <Tooltip text={PLAYGROUND_TOOLTIP} />
          {/if}
        </a>
        <!-- eslint-enable svelte/no-navigation-without-resolve -->
        {#if !canHover.current}
          <p class="playground-help text--small" data-test-playground-help>{PLAYGROUND_TOOLTIP}</p>
        {/if}
      {/if}
    {:catch}
      <!-- Silently ignore playground loading failures -->
    {/await}

    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
    <a href={reportUrl} data-test-id="link-crate-report" class="report-button button button--red button--small">
      Report crate
    </a>
  </div>
</section>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;

    > * + * {
      margin-top: var(--space-m);
    }
  }

  .heading {
    font-size: 1.17em;
    margin: 0 0 var(--space-s);
  }

  .metadata {
    > * + * {
      margin-top: var(--space-2xs);
    }
  }

  .date,
  .msrv,
  .edition,
  .license,
  .linecount,
  .bytes,
  .purl {
    display: flex;
    align-items: center;

    :global(svg) {
      flex-shrink: 0;
      margin-right: var(--space-2xs);
      height: 1em;
      width: auto;
    }
  }

  .date,
  .msrv,
  .edition,
  .linecount {
    > span {
      cursor: help;
    }
  }

  .license {
    :global(a) {
      color: var(--main-color);
    }
  }

  .linecount,
  .bytes {
    font-variant-numeric: tabular-nums;
  }

  .purl {
    align-items: flex-start;
  }

  .sidebar :global(.purl-copy-button) {
    text-align: left;
    width: 100%;
    min-width: 0;
    cursor: pointer;

    &:focus {
      outline: 2px solid var(--yellow500);
      outline-offset: 1px;
      border-radius: var(--space-3xs);
    }
  }

  .purl-text {
    word-break: break-all;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  .purl-tooltip {
    word-break: break-all;

    > small {
      word-break: normal;
    }
  }

  .purl-help-link {
    color: unset;
    margin-left: var(--space-2xs);
    flex-shrink: 0;

    &:hover {
      color: unset;
    }

    &:focus {
      outline: 2px solid var(--yellow500);
      outline-offset: 1px;
      border-radius: var(--space-3xs);
    }

    :global(svg) {
      margin: 0;
    }
  }

  .links {
    > :global(* + *) {
      margin-top: var(--space-m);
    }
  }

  .categories {
    margin: 0;
    padding-left: 20px;
    line-height: 1.5;
  }

  .report-button,
  .playground-button {
    justify-content: center;
    width: 220px;
  }

  .playground-button {
    display: flex;
    margin-bottom: var(--space-2xs);
  }

  .playground-help {
    max-width: 220px;
    text-align: justify;
    line-height: 1.3em;
  }
</style>
