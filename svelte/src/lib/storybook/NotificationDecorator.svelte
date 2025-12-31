<script lang="ts">
  import type { Snippet } from 'svelte';

  import NotificationContainer from '$lib/components/notifications/NotificationContainer.svelte';
  import { NotificationsState, setNotifications } from '$lib/notifications.svelte';

  let { children }: { children: Snippet } = $props();

  let notifications = new NotificationsState();
  setNotifications(notifications);
</script>

{@render children()}

<NotificationContainer />
