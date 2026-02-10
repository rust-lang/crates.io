<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import TrashIcon from '$lib/assets/trash.svg?component';
  import FollowButton from '$lib/components/FollowButton.svelte';
  import * as NavTabs from '$lib/components/nav-tabs';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { getSession } from '$lib/utils/session.svelte';

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Keyword = components['schemas']['Keyword'];

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
  }

  let { crate, version, versionNum: version_num, keywords = [] }: Props = $props();
  let crate_id = $derived(crate.id);

  let session = getSession();

  // TODO: implement isOwner check using session service
  let isOwner = $derived(false);

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
  <h1 class="heading">
    <span data-test-crate-name>{crate.name}</span>
    {#if version}
      <small data-test-crate-version>v{version.num}</small>

      {#if version.yanked}
        <span class="yanked-badge" data-test-yanked>
          <TrashIcon /> Yanked

          <Tooltip>
            This crate has been yanked, but it is still available for download for other crates that may be depending on
            it.
          </Tooltip>
        </span>
      {/if}
    {/if}
  </h1>

  {#if crate.description}
    <div class="description">
      {crate.description}
    </div>
  {/if}

  {#if keywords.length > 0}
    <ul class="keywords">
      {#each keywords as keyword (keyword.id)}
        <li>
          <a href={resolve('/keywords/[keyword_id]', { keyword_id: keyword.id })} data-test-keyword={keyword.id}>
            <span class="hash">#</span>{keyword.id}
          </a>
        </li>
      {/each}
    </ul>
  {/if}

  {#if session.currentUser}
    <div class="follow-button">
      <FollowButton crateName={crate.name} />
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
  {#if isOwner}
    <NavTabs.Tab href={settingsHref} data-test-settings-tab>Settings</NavTabs.Tab>
  {/if}
</NavTabs.Root>

<style>
  .header {
    padding: var(--space-s) var(--space-m);
    background-color: var(--main-bg-dark);
    margin-bottom: var(--space-s);
    border-radius: 5px;
  }

  .heading {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--space-xs);
    margin: 0;
    padding: 0;
    word-break: break-word;

    small {
      color: var(--main-color-light);
    }
  }

  .yanked-badge {
    background: #d30000;
    border-radius: 99999px;
    padding: var(--space-3xs) var(--space-s);
    font-size: var(--space-s);
    color: white;
    align-self: center;
    display: inline-flex;
    align-items: center;
    gap: var(--space-3xs);
    white-space: nowrap;
    cursor: default;

    :global(svg) {
      width: 1em;
      height: 1em;
      flex-shrink: 0;
    }
  }

  .description {
    margin-top: var(--space-xs);
    line-height: 1.35;
  }

  .keywords {
    list-style: none;
    margin: var(--space-xs) 0 0;
    padding: 0;

    > * {
      display: inline;

      + * {
        margin-left: var(--space-s);
      }
    }
  }

  .hash {
    margin-right: 1px;
    font-family: var(--font-monospace);
    font-size: 90%;
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
