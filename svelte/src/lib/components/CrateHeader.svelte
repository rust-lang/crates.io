<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import CrateFollowButton from '$lib/components/CrateFollowButton.svelte';
  import * as NavTabs from '$lib/components/nav-tabs';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { getSession } from '$lib/utils/session.svelte';

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Keyword = components['schemas']['Keyword'];
  type Owner = components['schemas']['Owner'];

  interface Props {
    /** The crate to display in the header. */
    crate: Crate;

    /** The version to display in the header (shows version number and yanked badge). */
    version?: Version;

    /**
     * The version number used for building navigation links (Readme, Dependencies tabs).
     *
     * This is separate from `version.num` because on the default crate details page we
     * display version info but want navigation links to point to the default routes,
     * while on version-specific pages we set both to make links point to that version.
     */
    versionNum?: string;

    /** The keywords associated with this crate. */
    keywords?: Keyword[];

    /** The owners of this crate, used to determine if the Settings tab should be shown. */
    ownersPromise?: Promise<Owner[]>;
  }

  let { crate, version, versionNum: version_num, keywords = [], ownersPromise }: Props = $props();
  let crate_id = $derived(crate.id);

  let session = getSession();

  let readmeHref = $derived(
    version_num
      ? resolve('/crates/[crate_id]/[version_num]', { crate_id, version_num })
      : resolve('/crates/[crate_id]', { crate_id }),
  );

  let versionsHref = $derived(resolve('/crates/[crate_id]/versions', { crate_id }));

  let depsHref = $derived(
    version_num
      ? resolve('/crates/[crate_id]/[version_num]/dependencies', { crate_id, version_num })
      : resolve('/crates/[crate_id]/dependencies', { crate_id }),
  );

  let revDepsHref = $derived(resolve('/crates/[crate_id]/reverse_dependencies', { crate_id }));

  let securityHref = $derived(resolve('/crates/[crate_id]/security', { crate_id }));

  let settingsHref = $derived(resolve('/crates/[crate_id]/settings', { crate_id }));
</script>

<div class="header" data-test-heading>
  <div class="title-row">
    <h1 class="heading">
      <span data-test-crate-name>{crate.name}</span>
      {#if version}
        <small data-test-crate-version>v{version.num}</small>
      {/if}
    </h1>

    {#if version?.yanked}
      <span class="yanked-badge" data-test-yanked>
        Yanked

        <Tooltip>
          This crate has been yanked, but it is still available for download for other crates that may be depending on
          it.
        </Tooltip>
      </span>
    {/if}
  </div>

  {#if crate.description}
    <div class="description">
      {crate.description}
    </div>
  {/if}

  {#if keywords.length !== 0}
    <ul class="keywords" aria-label="Keywords">
      {#each keywords as keyword (keyword.id)}
        <li>
          <a href={resolve('/keywords/[keyword_id]', { keyword_id: keyword.id })} data-test-keyword={keyword.id}>
            <span class="hash" aria-hidden="true">#</span>{keyword.id}
          </a>
        </li>
      {/each}
    </ul>
  {/if}

  {#if session.currentUser}
    <div class="follow-button">
      <CrateFollowButton crateName={crate.name} />
    </div>
  {/if}
</div>

<NavTabs.Root aria-label="{crate.name} crate subpages" style="margin-bottom: var(--space-s)">
  <NavTabs.Tab href={readmeHref} data-test-readme-tab>Readme</NavTabs.Tab>
  <NavTabs.Tab href={versionsHref} data-test-versions-tab>
    {crate.num_versions}
    {crate.num_versions === 1 ? 'Version' : 'Versions'}
  </NavTabs.Tab>
  <NavTabs.Tab href={depsHref} data-test-deps-tab>Dependencies</NavTabs.Tab>
  <NavTabs.Tab href={revDepsHref} data-test-rev-deps-tab>Dependents</NavTabs.Tab>
  <NavTabs.Tab href={securityHref} data-test-security-tab>Security</NavTabs.Tab>
  {#await ownersPromise then owners}
    {#if owners?.some(o => o.kind === 'user' && o.id === session.currentUser?.id)}
      <NavTabs.Tab href={settingsHref} data-test-settings-tab>Settings</NavTabs.Tab>
    {/if}
  {/await}
</NavTabs.Root>

<style>
  .header {
    --shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);

    padding: var(--space-s) var(--space-m);
    background-color: light-dark(white, #141413);
    margin-bottom: var(--space-s);
    border-radius: 5px;
    box-shadow: var(--shadow);
  }

  .heading {
    display: inline-flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--space-xs);
    margin: 0;
    padding: 0;
    word-break: break-word;
    font-size: var(--space-l);

    small {
      font-size: var(--space-m);
      font-weight: 500;
      color: var(--main-color-light);
    }
  }

  .yanked-badge {
    display: inline-block;
    margin-left: var(--space-2xs);
    margin-top: var(--space-2xs);
    /* Suppress the text-derived baseline so the synthesized baseline is the
       pill's bottom edge, letting it sit on the title's baseline. */
    overflow: hidden;

    background: light-dark(oklch(0.9 0.02 24), oklch(0.25 0.03 24));
    border-radius: 99999px;
    padding: var(--space-4xs) var(--space-2xs);
    font-size: calc(0.9 * var(--space-xs));
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: light-dark(oklch(0.5 0.15 24), oklch(0.8 0.07 24));
    white-space: nowrap;
    cursor: default;
  }

  .description {
    margin-top: var(--space-xs);
    font-size: calc(0.9 * var(--space-s));
    line-height: 1.35;
  }

  .keywords {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2xs);
    margin: var(--space-xs) 0 0;
    padding: 0;
    font-size: calc(0.85 * var(--space-s));
    overflow: hidden;

    a {
      display: inline-flex;
      align-items: center;
      gap: var(--space-4xs);
      padding: var(--space-4xs) var(--space-xs);
      color: var(--main-color-light);
      background: var(--main-bg);
      border-radius: 99999px;
      white-space: nowrap;
      transition: color var(--transition-fast);

      &:hover {
        color: var(--main-color);
      }
    }
  }

  .hash {
    font-family: var(--font-monospace);
    color: var(--main-color-light);
    opacity: 0.65;
  }

  .follow-button {
    margin-top: var(--space-s);
  }

  @media only screen and (min-width: 751px) {
    .header {
      display: grid;
      grid-template-columns: 1fr auto;
    }

    .follow-button {
      margin: -10px -10px 0 var(--space-s);
      grid-column: 2;
      grid-row: 1;
    }

    .description,
    .keywords {
      grid-column: 1 / 3;
    }
  }
</style>
