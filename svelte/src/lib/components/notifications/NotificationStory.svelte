<script lang="ts">
  import type { NotificationsContext } from '$lib/notifications.svelte';
  import type { Snippet } from 'svelte';

  import { getNotifications } from '$lib/notifications.svelte';

  interface Props {
    children: Snippet<[NotificationsContext]>;
  }

  let { children }: Props = $props();

  let notifications = getNotifications();
</script>

{@render children(notifications)}
