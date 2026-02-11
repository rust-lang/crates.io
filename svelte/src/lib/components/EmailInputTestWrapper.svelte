<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { NotificationsState, setNotifications } from '$lib/notifications.svelte';
  import EmailInput from './EmailInput.svelte';
  import NotificationContainer from './notifications/NotificationContainer.svelte';

  type AuthenticatedUser = components['schemas']['AuthenticatedUser'];

  interface Props {
    user: AuthenticatedUser;
  }

  let { user: initialUser }: Props = $props();

  // svelte-ignore state_referenced_locally
  let user: AuthenticatedUser = $state(initialUser);

  let notifications = new NotificationsState();
  setNotifications(notifications);
</script>

<EmailInput bind:user />
<NotificationContainer />
