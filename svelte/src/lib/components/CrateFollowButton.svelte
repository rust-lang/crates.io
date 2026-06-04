<script lang="ts">
  import { createClient } from '@crates-io/api-client';

  import FollowButton from '$lib/components/FollowButton.svelte';
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
  let status = $derived<'loading' | 'following' | 'not-following'>(
    isLoading ? 'loading' : following ? 'following' : 'not-following',
  );

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

<FollowButton {status} disabled={loadError} onclick={toggleFollow} />
