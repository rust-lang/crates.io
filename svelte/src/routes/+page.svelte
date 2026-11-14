<script lang="ts">
  import type { PageProps } from './$types';

  import { invalidateAll } from '$app/navigation';

  import BlogPosts from '#lib/components/frontpage/BlogPosts.svelte';
  import CrateLists from '#lib/components/frontpage/CrateLists.svelte';
  import ErrorState from '#lib/components/frontpage/ErrorState.svelte';
  import HeroButtons from '#lib/components/frontpage/HeroButtons.svelte';
  import IntroBlurb from '#lib/components/frontpage/IntroBlurb.svelte';

  let { data }: PageProps = $props();

  let isFirstLoad = $state(true);

  async function retry() {
    isFirstLoad = false;
    await invalidateAll();
  }
</script>

<HeroButtons />

{#await data.summary}
  <IntroBlurb />
{:then summary}
  <IntroBlurb {summary} />
  <!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
{:catch _error}
  <IntroBlurb />
{/await}

{#await data.blogPosts}
  <BlogPosts state={{ status: 'loading' }} />
{:then posts}
  <BlogPosts state={{ status: 'loaded', posts }} />
{:catch}
  <BlogPosts state={{ status: 'error' }} />
{/await}

{#await data.summary}
  {#if isFirstLoad}
    <!-- during first load, show placeholders -->
    <CrateLists />
  {:else}
    <!-- during retries, show error state -->
    <ErrorState isLoading={true} />
  {/if}
{:then summary}
  <CrateLists {summary} />
  <!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
{:catch _error}
  <ErrorState isLoading={false} onRetry={retry} />
{/await}
