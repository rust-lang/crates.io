<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';
  import { formatDistanceToNow } from 'date-fns';

  import DownloadIcon from '$lib/assets/download.svg?component';
  import FollowingIcon from '$lib/assets/following.svg?component';
  import LatestUpdatesIcon from '$lib/assets/latest-updates.svg?component';
  import MyPackagesIcon from '$lib/assets/my-packages.svg?component';
  import CrateDownloadsList from '$lib/components/CrateDownloadsList.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';

  type Version = components['schemas']['Version'];

  let { data } = $props();

  const TO_SHOW = 5;
  const numberFormat = new Intl.NumberFormat();

  let visibleCrates = $derived(data.myCrates.slice(0, TO_SHOW));
  let visibleFollowing = $derived(data.myFollowing.slice(0, TO_SHOW));
  let hasMoreCrates = $derived(data.myCrates.length > TO_SHOW);
  let hasMoreFollowing = $derived(data.myFollowing.length > TO_SHOW);

  let extraVersions: Version[] = $state([]);
  let extraHasMore: boolean | undefined = $state();
  let loading = $state(false);

  let feed = $derived([...data.updates.versions, ...extraVersions]);
  let hasMore = $derived(extraHasMore ?? data.updates.meta.more);

  async function loadMore() {
    loading = true;
    try {
      let page = feed.length / 10 + 1;
      let response = await fetch(`/api/v1/me/updates?page=${page}`);
      let json = await response.json();
      extraVersions = [...extraVersions, ...json.versions];
      extraHasMore = json.meta.more;
    } finally {
      loading = false;
    }
  }
</script>

<PageTitle title="Dashboard" />

<PageHeader>
  <div class="page-header-content">
    <h1>My Dashboard</h1>
    <div class="stats">
      <div class="downloads">
        <DownloadIcon class="header-icon" />
        <span class="num">{numberFormat.format(data.totalDownloads)}</span>
        <span class="stats-label text--small">Total Downloads</span>
      </div>
    </div>
  </div>
</PageHeader>

<div class="my-info">
  <div class="my-crate-lists">
    <div class="section-header">
      <h2>
        <MyPackagesIcon />
        My Crates
      </h2>

      {#if hasMoreCrates}
        <a href={resolve('/users/[user_id]', { user_id: data.user.login })} class="show-all-link">Show all</a>
      {/if}
    </div>
    <CrateDownloadsList crates={visibleCrates} />

    <div class="section-header">
      <h2>
        <FollowingIcon />
        Following
      </h2>

      {#if hasMoreFollowing}
        <a href={resolve('/me/following')} class="show-all-link">Show all</a>
      {/if}
    </div>
    <CrateDownloadsList crates={visibleFollowing} />
  </div>

  <div class="my-feed">
    <h2>
      <LatestUpdatesIcon />
      Latest Updates
    </h2>

    <div class="feed">
      <ul class="feed-list" data-test-feed-list>
        {#each feed as version (version.id)}
          <li class="feed-row">
            <a
              href={resolve('/crates/[crate_id]/[version_num]', { crate_id: version.crate, version_num: version.num })}
            >
              {version.crate}
              <span class="text--small">{version.num}</span>
            </a>
            <span class="feed-date text--small">
              {formatDistanceToNow(version.created_at, { addSuffix: true })}
            </span>
          </li>
        {/each}
      </ul>

      {#if hasMore}
        <div class="load-more">
          <button type="button" class="load-more-button" disabled={loading} onclick={loadMore}>
            Load More
            {#if loading}
              <LoadingSpinner />
            {/if}
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .page-header-content {
    display: flex;
    align-items: center;
  }

  .page-header-content :global(svg.header-icon) {
    flex-shrink: 0;
    margin-right: var(--space-2xs);
    width: 32px;
    height: 32px;
  }

  .stats {
    margin-left: auto;

    .num {
      font-size: 30px;
      font-weight: bold;
    }

    .downloads {
      display: flex;
      align-items: center;
    }
  }

  .stats-label {
    margin-left: var(--space-2xs);
  }

  .my-info {
    display: flex;
    gap: var(--space-s);

    h2 {
      display: flex;
      align-items: center;
      gap: var(--space-3xs);
      font-size: 1.05em;
      margin: 0;

      > :global(*) {
        flex-shrink: 0;
      }
    }

    @media only screen and (max-width: 750px) {
      flex-direction: column;
    }
  }

  .my-crate-lists {
    flex-direction: column;
    flex-grow: 2;

    .section-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
    }

    > :global(ul) {
      margin: var(--space-s) 0;
    }

    @media only screen and (max-width: 750px) {
      order: 1;
    }
  }

  .show-all-link {
    color: var(--main-color-light);
    text-decoration: underline;
    font-size: 90%;
    font-weight: normal;

    &:hover {
      color: #6b6b6b;
    }
  }

  .my-feed {
    flex-grow: 5;

    @media only screen and (max-width: 750px) {
      order: 0;
    }
  }

  .feed {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
    margin: var(--space-s) 0;
  }

  .feed-list {
    list-style: none;
    margin: 0;
    padding: 0;

    > * {
      display: flex;
      align-items: baseline;
      padding: var(--space-s);
    }

    > * + * {
      border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
    }
  }

  .feed-date {
    flex-grow: 1;
    text-align: right;
  }

  .load-more {
    padding: var(--space-s);
    border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
  }
</style>
