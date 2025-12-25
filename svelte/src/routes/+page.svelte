<script lang="ts">
  import type { PageProps } from './$types';

  import { invalidateAll } from '$app/navigation';

  import CrateLists from '$lib/components/frontpage/CrateLists.svelte';
  import ErrorState from '$lib/components/frontpage/ErrorState.svelte';
  import HeroButtons from '$lib/components/frontpage/HeroButtons.svelte';
  import IntroBlurb from '$lib/components/frontpage/IntroBlurb.svelte';

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
  {#if isFirstLoad}
    <!-- during first load, show placeholders -->
    <CrateLists />
  {:else}
    <!-- during retries, show error state -->
    <ErrorState isLoading={true} />
  {/if}
{:then summary}
  <IntroBlurb {summary} />
  <CrateLists {summary} />
{:catch _error}
  <IntroBlurb />
  <ErrorState isLoading={false} onRetry={retry} />
{/await}
