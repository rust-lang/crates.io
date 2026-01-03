<script lang="ts">
  import { getNotifications } from '$lib/notifications.svelte';
  import NotificationMessage from './NotificationMessage.svelte';

  type Position = 'top' | 'top-left' | 'top-right' | 'bottom' | 'bottom-left' | 'bottom-right';

  interface Props {
    position?: Position;
  }

  let { position = 'top-right' }: Props = $props();

  let notificationsContext = getNotifications();
</script>

<div class="container container--{position}" data-test-notification-container>
  {#each notificationsContext.content as notification (notification)}
    <NotificationMessage {notification} />
  {/each}
</div>

<style>
  .container {
    position: fixed;
    margin: 0 auto;
    width: 80%;
    max-width: 400px;
    z-index: 1000;
  }

  .container--top {
    top: 10px;
    right: 0;
    left: 0;
  }

  .container--top-left {
    top: 10px;
    right: auto;
    left: 10px;
  }

  .container--top-right {
    top: 10px;
    right: 10px;
    left: auto;
  }

  .container--bottom {
    right: 0;
    bottom: 10px;
    left: 0;
  }

  .container--bottom-left {
    right: auto;
    bottom: 10px;
    left: 10px;
  }

  .container--bottom-right {
    right: 10px;
    bottom: 10px;
    left: auto;
  }
</style>
