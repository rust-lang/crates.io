<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  import { getNotifications } from '$lib/notifications.svelte';
  import { getSession } from '$lib/utils/session.svelte';

  let { data } = $props();

  let notifications = getNotifications();
  let session = getSession();

  // Using `onMount` instead of `load()` because this is a mutation (PUT),
  // not data loading. `onMount` only runs in the browser, which avoids
  // issues with SSR and preloading re-firing the mutation.
  onMount(async () => {
    let emailToken = data.email_token;
    let client = createClient({ fetch });

    try {
      let result = await client.PUT('/api/v1/confirm/{email_token}', {
        params: { path: { email_token: emailToken } },
      });

      if (result.response.ok) {
        if (session.currentUser) {
          session.currentUser.email_verified = true;
        }

        notifications.success('Thank you for confirming your email! :)');
      } else {
        notifications.error('Unknown error in email confirmation');
      }
    } catch {
      notifications.error('Unknown error in email confirmation');
    }

    await goto(resolve('/'), { replaceState: true });
  });
</script>
