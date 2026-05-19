<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { HTMLAttributes } from 'svelte/elements';

  import { resolve } from '$app/paths';
  import { format, formatDistanceToNow, formatISO } from 'date-fns';
  import prettyBytes from 'pretty-bytes';
  import semverParse from 'semver/functions/parse';

  import CalendarIcon from '$lib/assets/calendar.svg?component';
  import CheckboxEmptyIcon from '$lib/assets/checkbox-empty.svg?component';
  import CheckboxIcon from '$lib/assets/checkbox.svg?component';
  import EllipsisCircleIcon from '$lib/assets/ellipsis-circle.svg?component';
  import GitHubIcon from '$lib/assets/github.svg?component';
  import GitLabIcon from '$lib/assets/gitlab.svg?component';
  import LicenseIcon from '$lib/assets/license.svg?component';
  import RustIcon from '$lib/assets/rust.svg?component';
  import TrashIcon from '$lib/assets/trash.svg?component';
  import WeightIcon from '$lib/assets/weight.svg?component';
  import Edition from '$lib/components/crate-sidebar/Edition.svelte';
  import Msrv from '$lib/components/crate-sidebar/Msrv.svelte';
  import * as Dropdown from '$lib/components/dropdown';
  import LicenseExpression from '$lib/components/LicenseExpression.svelte';
  import PrivilegedAction from '$lib/components/PrivilegedAction.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import UserAvatar from '$lib/components/UserAvatar.svelte';
  import YankButton from '$lib/components/YankButton.svelte';

  const EIGHT_DAYS = 8 * 24 * 60 * 60 * 1000;

  type Version = components['schemas']['Version'];
  type User = components['schemas']['User'];
  type Owner = components['schemas']['Owner'];

  interface Feature {
    name: string;
    isDefault: boolean;
    dependencies: string[];
  }

  interface Props extends HTMLAttributes<HTMLDivElement> {
    crateName: string;
    version: Version;
    isHighestOfReleaseTrack?: boolean;
    isOwner?: boolean;
    onVersionChanged?: () => void | Promise<void>;
  }

  let {
    crateName,
    version,
    isHighestOfReleaseTrack = false,
    isOwner = false,
    onVersionChanged,
    ...others
  }: Props = $props();

  let focused = $state(false);

  let semver = $derived(semverParse(version.num, { loose: true }));

  let isPrerelease = $derived(Boolean(semver?.prerelease.length));

  let releaseTrack = $derived.by(() => {
    if (!semver) return null;
    let major = semver.major;
    return major >= 100 ? String(major) : `${major}.${major === 0 ? semver.minor : 'x'}`;
  });

  let isNew = $derived(Date.now() - new Date(version.created_at).getTime() < EIGHT_DAYS);

  let featureList = $derived.by((): Feature[] => {
    let features = version.features as Record<string, string[]> | null | undefined;
    if (typeof features !== 'object' || features === null) {
      return [];
    }

    let defaultFeatures = features.default ?? [];
    return Object.keys(features)
      .filter(name => name !== 'default')
      .sort()
      .map(name => ({ name, isDefault: defaultFeatures.includes(name), dependencies: features[name] ?? [] }));
  });

  let features = $derived.by(() => {
    let list = featureList.slice(0, 15);
    let more = featureList.length - list.length;
    return { list, more };
  });

  let trustpubProvider = $derived((version.trustpub_data as { provider?: string } | null | undefined)?.provider);

  let trustpubPublisher = $derived.by(() => {
    if (trustpubProvider === 'github') return 'GitHub';
    if (trustpubProvider === 'gitlab') return 'GitLab';
    return null;
  });

  let trustpubUrl = $derived.by(() => {
    let data = version.trustpub_data as
      | { provider?: string; repository?: string; run_id?: string; project_path?: string; job_id?: string }
      | null
      | undefined;
    if (data?.provider === 'github' && data.repository && data.run_id) {
      return `https://github.com/${data.repository}/actions/runs/${data.run_id}`;
    }
    if (data?.provider === 'gitlab' && data.project_path && data.job_id) {
      return `https://gitlab.com/${data.project_path}/-/jobs/${data.job_id}`;
    }
    return null;
  });

  let publishedBy = $derived.by((): Owner | null => {
    let user = version.published_by as User | null | undefined;
    if (!user) return null;
    return { ...user, kind: 'user', url: user.url ?? null };
  });
</script>

<div
  class="row"
  class:latest={isHighestOfReleaseTrack}
  class:yanked={version.yanked}
  class:prerelease={isPrerelease}
  class:focused
  {...others}
>
  <div class="version">
    <div class="release-track" data-test-release-track>
      {#if version.yanked}
        <TrashIcon aria-hidden="true" />
        <span class="sr-only">Yanked</span>
      {:else if !semver}
        <span aria-hidden="true">?</span>
        <span class="sr-only">Unable to parse version</span>
      {:else}
        <span aria-hidden="true">{releaseTrack}</span>
        <span class="sr-only">
          Release track {releaseTrack}
          {#if isPrerelease || isHighestOfReleaseTrack}
            ({#if isPrerelease}prerelease{/if}{#if isPrerelease && isHighestOfReleaseTrack},
            {/if}{#if isHighestOfReleaseTrack}latest{/if})
          {/if}
        </span>
      {/if}

      <Tooltip side="right">
        <div class="rt-tooltip">
          {#if version.yanked}
            This version was
            <span class="rt-yanked">yanked</span>
          {:else if !semver}
            Failed to parse version
            {version.num}
          {:else}
            Release Track:
            {releaseTrack}
            {#if isPrerelease || isHighestOfReleaseTrack}
              ({#if isPrerelease}<span class="rt-prerelease">prerelease</span
                >{/if}{#if isPrerelease && isHighestOfReleaseTrack},&nbsp;{/if}{#if isHighestOfReleaseTrack}<span
                  class="rt-latest">latest</span
                >{/if})
            {/if}
          {/if}
        </div>
      </Tooltip>
    </div>

    <a
      href={resolve('/crates/[crate_id]/[version_num]', { crate_id: crateName, version_num: version.num })}
      class="num-link"
      onfocusin={() => (focused = true)}
      onfocusout={() => (focused = false)}
      data-test-release-track-link
    >
      {version.num}
    </a>
  </div>

  <div class="metadata" role="list" aria-label="Version metadata">
    <div class="metadata-row" role="presentation">
      {#if publishedBy}
        <span class="publisher" role="listitem">
          by
          <a href={resolve('/users/[user_id]', { user_id: publishedBy.login })}>
            <UserAvatar user={publishedBy} class="avatar" />
            {publishedBy.name ?? publishedBy.login}
          </a>
        </span>
      {:else if trustpubPublisher}
        <span class="publisher trustpub" role="listitem">
          via
          {#if trustpubUrl}
            <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
            <a href={trustpubUrl} target="_blank" rel="nofollow noopener noreferrer">
              {#if trustpubProvider === 'github'}
                <GitHubIcon aria-hidden="true" />
              {:else if trustpubProvider === 'gitlab'}
                <GitLabIcon aria-hidden="true" />
              {/if}
              {trustpubPublisher}
            </a>
          {:else}
            {#if trustpubProvider === 'github'}
              <GitHubIcon aria-hidden="true" />
            {:else if trustpubProvider === 'gitlab'}
              <GitLabIcon aria-hidden="true" />
            {/if}
            {trustpubPublisher}
          {/if}
        </span>
      {/if}

      <time datetime={formatISO(version.created_at)} class="date" class:new={isNew} role="listitem">
        <CalendarIcon aria-hidden="true" />
        {formatDistanceToNow(version.created_at, { addSuffix: true })}

        <Tooltip>
          {format(version.created_at, 'PPP')}
          {#if isNew}
            (<span class="new">new</span>)
          {/if}
        </Tooltip>
      </time>
    </div>

    {#if version.crate_size || version.license || featureList.length !== 0}
      <div class="metadata-row" role="presentation">
        {#if version.rust_version}
          <span class="msrv" role="listitem">
            <RustIcon aria-hidden="true" />
            <span class="sr-only">Minimum Rust version:</span>
            <Msrv msrv={version.rust_version} edition={version.edition} />
          </span>
        {:else if version.edition}
          <span class="edition" role="listitem">
            <RustIcon aria-hidden="true" />
            <Edition edition={version.edition} />
          </span>
        {/if}

        {#if version.crate_size}
          <span class="bytes" role="listitem">
            <WeightIcon aria-hidden="true" />
            <span class="sr-only">Size:</span>
            {prettyBytes(version.crate_size, { binary: true })}
          </span>
        {/if}

        {#if version.license}
          <span class="license" role="listitem">
            <LicenseIcon aria-hidden="true" />
            <span class="sr-only">License:</span>
            <LicenseExpression license={version.license} />
          </span>
        {/if}

        {#if featureList.length !== 0}
          <span class="num-features" role="listitem" data-test-feature-list>
            <CheckboxIcon aria-hidden="true" />
            {featureList.length}
            {featureList.length === 1 ? 'Feature' : 'Features'}

            <Tooltip>
              <ul class="feature-list">
                {#each features.list as feature (feature.name)}
                  <li>
                    {#if feature.isDefault}
                      <CheckboxIcon aria-hidden="true" />
                    {:else}
                      <CheckboxEmptyIcon aria-hidden="true" />
                    {/if}
                    {feature.name}
                    {#if feature.isDefault}<span class="sr-only">(default)</span>{/if}
                  </li>
                {/each}
                {#if features.more > 0}
                  <li class="other-features">
                    and
                    {features.more}
                    other features
                  </li>
                {/if}
              </ul>
            </Tooltip>
          </span>
        {/if}
      </div>
    {/if}
  </div>

  <PrivilegedAction userAuthorised={isOwner} class="actions">
    <Dropdown.Root class="dropdown" data-test-actions-menu>
      <Dropdown.Trigger hideArrow class="trigger" data-test-actions-toggle>
        <EllipsisCircleIcon class="icon" aria-hidden="true" />
        <span class="sr-only">Actions</span>
      </Dropdown.Trigger>

      <Dropdown.Menu class="menu">
        <Dropdown.Item>
          <YankButton {crateName} {version} onChanged={onVersionChanged} class="button-reset menu-button" />
        </Dropdown.Item>
        <Dropdown.Item>
          <a
            href={resolve('/crates/[crate_id]/[version_num]/rebuild-docs', {
              crate_id: crateName,
              version_num: version.num,
            })}
            class="button-reset menu-button"
            data-test-id="btn-rebuild-docs"
          >
            Rebuild Docs
          </a>
        </Dropdown.Item>
      </Dropdown.Menu>
    </Dropdown.Root>
  </PrivilegedAction>
</div>

<style>
  .row {
    --bg-color: light-dark(var(--grey200), #242422);
    --hover-bg-color: light-dark(hsl(217, 37%, 98%), hsl(204, 3%, 11%));
    --fg-color: light-dark(var(--grey700), #ccc);
    --shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);

    display: flex;
    align-items: center;
    position: relative;
    font-size: 18px;
    padding: var(--space-s) var(--space-m);
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: var(--shadow);
    transition: all var(--transition-slow);

    &:hover,
    &.focused {
      background-color: var(--hover-bg-color);
      transition: all var(--transition-instant);
    }

    &.focused {
      box-shadow:
        0 0 0 3px var(--yellow500),
        var(--shadow);
    }

    &.latest {
      --bg-color: light-dark(hsl(109, 75%, 87%), hsl(136, 67%, 11%));
      --hover-bg-color: light-dark(hsl(109, 75%, 97%), hsl(109, 10%, 11%));
      --fg-color: light-dark(hsl(136, 67%, 38%), hsl(109, 75%, 87%));
    }

    &.prerelease {
      --bg-color: light-dark(hsl(39, 100%, 91%), hsl(39, 71%, 15%));
      --hover-bg-color: light-dark(hsl(39, 100%, 97%), hsl(39, 10%, 11%));
      --fg-color: light-dark(hsl(39, 71%, 45%), hsl(39, 100%, 91%));
    }

    &.yanked {
      --bg-color: light-dark(hsl(0, 92%, 90%), hsl(0, 84%, 12%));
      --hover-bg-color: light-dark(hsl(0, 92%, 98%), hsl(0, 10%, 11%));
      --fg-color: light-dark(hsl(0, 84%, 32%), hsl(0, 92%, 90%));
    }

    :global(.actions) {
      display: flex;
    }

    :global(.dropdown) {
      display: flex;
      font-size: initial;
      line-height: 1rem;
    }

    :global(.icon) {
      width: 2em;
      height: auto;
    }

    :global(.trigger) {
      background: none;
      border: none;
      padding: 0;
      border-radius: 99999px;
      color: var(--grey600);

      &:hover {
        border-radius: 99999px;
        color: var(--grey900);
        background-color: white;
      }
    }

    :global(.menu) {
      top: 100%;
      right: 0;
      min-width: max-content;
    }

    :global(.menu-button) {
      align-items: center;
      gap: var(--space-2xs);
      cursor: pointer;
      text-transform: capitalize;

      &[disabled] {
        background: linear-gradient(to bottom, var(--bg-color-top-light) 0%, var(--bg-color-bottom-light) 100%);
        color: var(--disabled-text-color) !important;
        cursor: not-allowed;
      }
    }
  }

  .release-track,
  .date,
  .num-features {
    z-index: 1;
    cursor: help;
  }

  .date,
  .num-features {
    position: relative;
  }

  .version {
    display: grid;
    grid-template-columns: auto auto;
    place-items: center;

    @media only screen and (max-width: 550px) {
      grid-template-columns: auto;
      margin: 0 var(--space-s);
    }
  }

  .release-track {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: var(--space-xl);
    height: var(--space-xl);
    overflow: hidden;
    margin-right: var(--space-s);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    color: var(--fg-color);
    background-color: var(--bg-color);
    border: 1px solid light-dark(white, #808080);
    border-radius: 50%;
    transition: all var(--transition-fast);

    & > :global(svg) {
      height: 1em;
      width: auto;
    }

    .row:hover &,
    .row.focused & {
      border: var(--space-4xs) solid light-dark(white, #bfbfbf);
      box-shadow: 0 1px 3px light-dark(var(--fg-color), #232321);
    }

    @media only screen and (max-width: 550px) {
      margin: 0 0 var(--space-s);
    }
  }

  .rt-latest {
    color: hsl(136, 67%, 38%);
  }

  .rt-prerelease {
    color: hsl(35, 95%, 59%);
  }

  .rt-yanked {
    color: hsl(0, 87%, 58%);
  }

  .rt-tooltip {
    word-break: break-all;
  }

  .num-link {
    max-width: 200px;
    text-overflow: ellipsis;
    overflow: hidden;
    color: var(--fg-color);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    outline: none;

    &:hover {
      color: var(--fg-color);
    }

    &::after {
      content: '';
      position: absolute;
      left: 0;
      top: 0;
      right: 0;
      bottom: 0;
    }
  }

  .metadata {
    flex-grow: 1;
    margin-left: var(--space-m);
    color: light-dark(var(--grey600), #d1cfc7);
    text-transform: uppercase;
    letter-spacing: 0.7px;
    font-size: 13px;

    :global(a) {
      position: relative;
      color: inherit;

      &:hover {
        color: light-dark(var(--grey900), #f5f3e9);
      }

      &:focus-visible {
        outline: none;
        color: var(--yellow500);
      }
    }

    :global(svg) {
      height: 1em;
      width: auto;
      margin-right: var(--space-4xs);
      margin-bottom: -0.1em;
    }

    > * + * {
      margin-top: var(--space-2xs);

      @media only screen and (max-width: 750px) {
        margin-top: var(--space-xs);
      }
    }
  }

  .metadata :global(.avatar) {
    height: 1.5em;
    width: auto;
    margin-left: var(--space-4xs);
    margin-bottom: -0.4em;
    border-radius: 50%;
    box-shadow: 0 1px 1px 0 var(--grey600);
    padding: 1px;
  }

  .metadata-row {
    > * + * {
      margin-left: var(--space-s);
    }

    @media only screen and (max-width: 750px) {
      display: flex;
      flex-direction: column;
      align-items: flex-start;

      > * + * {
        margin-left: 0;
        margin-top: var(--space-xs);
      }
    }
  }

  .new {
    color: hsl(39, 98%, 47%);
  }

  .msrv {
    text-transform: initial;
  }

  .msrv,
  .edition {
    :global(svg) {
      margin-bottom: -0.15em;
    }
  }

  .bytes {
    font-variant-numeric: tabular-nums;
    text-transform: none;
  }

  .feature-list {
    padding: 0;
    margin: var(--space-2xs) var(--space-3xs);
    list-style: none;

    :global(svg) {
      height: 1em;
      width: auto;
      margin-right: var(--space-4xs);
      margin-bottom: -0.1em;
    }
  }

  .other-features {
    font-style: italic;
    margin-top: var(--space-2xs);
  }
</style>
