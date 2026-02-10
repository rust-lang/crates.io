<script lang="ts">
  import { createClient } from '@crates-io/api-client';

  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import { getNotifications } from '$lib/notifications.svelte';

  interface Props {
    /** The name of the crate to follow/unfollow. */
    crateName: string;
  }

  let { crateName }: Props = $props();

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let following = $state(false);
  let isLoadingState = $state(true);
  let isToggling = $state(false);
  let loadError = $state(false);

  let isLoading = $derived(isLoadingState || isToggling);
  let isDisabled = $derived(isLoading || loadError);

  $effect(() => {
    loadFollowState(crateName);
  });

  async function loadFollowState(name: string) {
    isLoadingState = true;
    loadError = false;

    try {
      let result = await client.GET('/api/v1/crates/{name}/following', {
        params: { path: { name } },
      });

      if (!result.response.ok) {
        throw new Error(`Failed to load follow state: ${result.response.status}`);
      }

      following = result.data!.following;
    } catch {
      loadError = true;
      notifications.error(
        `Something went wrong while trying to figure out if you are already following the ${name} crate. Please try again later!`,
      );
    } finally {
      isLoadingState = false;
    }
  }

  async function toggleFollow() {
    isToggling = true;

    try {
      let options = { params: { path: { name: crateName } } };

      let result = following
        ? await client.DELETE('/api/v1/crates/{name}/follow', options)
        : await client.PUT('/api/v1/crates/{name}/follow', options);

      if (!result.response.ok) {
        throw new Error(`Failed to ${following ? 'unfollow' : 'follow'} crate: ${result.response.status}`);
      }

      following = !following;
    } catch {
      notifications.error(
        `Something went wrong when ${following ? 'unfollowing' : 'following'} the ${crateName} crate. Please try again later!`,
      );
    } finally {
      isToggling = false;
    }
  }
</script>

<button
  type="button"
  disabled={isDisabled}
  data-test-follow-button
  class="follow-button button button--tan"
  onclick={toggleFollow}
>
  {#if isLoading}
    <LoadingSpinner theme="light" data-test-spinner />
  {:else if following}
    Unfollow
  {:else}
    Follow
  {/if}
</button>

<style>
  .follow-button {
    height: 48px;
    width: 150px;
    justify-content: center;
  }
</style>
