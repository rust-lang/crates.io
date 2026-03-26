<script lang="ts">
  import { navigating, page } from '$app/state';
  import { createClient } from '@crates-io/api-client';

  import { ColorSchemeState, setColorScheme } from '$lib/color-scheme.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import Header from '$lib/components/Header.svelte';
  import NotificationContainer from '$lib/components/notifications/NotificationContainer.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import TooltipContainer from '$lib/components/TooltipContainer.svelte';
  import { NotificationsState, setNotifications } from '$lib/notifications.svelte';
  import { PageTitleState, setPageTitle } from '$lib/page-title.svelte';
  import { ProgressState, setProgressContext } from '$lib/progress.svelte';
  import { SearchFormContext, setSearchFormContext } from '$lib/search-form.svelte';
  import { setTooltipContext } from '$lib/tooltip.svelte';
  import { SessionState, setSession } from '$lib/utils/session.svelte';

  import '$lib/css/global.css';

  let { children, data } = $props();
  let propsId = $props.id();

  let isIndex = $derived(page.route.id === '/');

  let colorScheme = new ColorSchemeState();
  setColorScheme(colorScheme);

  $effect(() => {
    document.documentElement.dataset.colorScheme = colorScheme.resolvedScheme;
  });

  let pageTitle = new PageTitleState();
  setPageTitle(pageTitle);

  let searchFormContext = new SearchFormContext();
  setSearchFormContext(searchFormContext);

  let progress = new ProgressState();
  setProgressContext(progress);

  $effect(() => {
    if (navigating.complete) {
      progress.trackPromise(navigating.complete);
    }
  });

  setTooltipContext({ containerId: `tooltip-container-${propsId}` });

  let notifications = new NotificationsState();
  setNotifications(notifications);

  let sessionState = new SessionState(createClient({ fetch }), notifications);
  setSession(sessionState);

  // svelte-ignore state_referenced_locally
  sessionState.initialPromise = data.userPromise.then(user => sessionState.setUser(user));

  const READ_ONLY_MESSAGE =
    'crates.io is currently in read-only mode for maintenance reasons. Some functionality will be temporarily unavailable.';

  data.siteMetadataPromise
    .then(response => {
      if (!response.data) return;
      let { read_only, banner_message } = response.data;
      let message = banner_message ?? (read_only ? READ_ONLY_MESSAGE : undefined);
      if (message) {
        notifications.info(message, { autoClear: false });
      }
    })
    .catch(() => {});
</script>

<svelte:head>
  <title>{pageTitle.title}</title>
</svelte:head>

{#if !__TEST__}
  <!-- Disabled in tests to ensure stable snapshots -->
  <ProgressBar />
{/if}

<NotificationContainer position="top-right" />
<TooltipContainer />

<Header hero={isIndex} />

<main class="main">
  <div class="inner-main width-limit">
    {@render children()}
  </div>
</main>

<Footer />

<style>
  .main {
    flex-grow: 1;
    display: flex;
    justify-content: center;
    width: 100%;
    position: relative;
    background-color: var(--main-bg);
    color: var(--main-color);
    box-shadow: 0 0 6px 0 var(--main-shadow-color);
  }

  .inner-main {
    --main-layout-padding: var(--space-s);

    display: flex;
    flex-direction: column;
    padding: var(--main-layout-padding);
  }
</style>
