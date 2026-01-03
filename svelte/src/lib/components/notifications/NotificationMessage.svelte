<script lang="ts">
  import type { Notification } from '$lib/notifications.svelte';

  import { getNotifications } from '$lib/notifications.svelte';
  import IconClose from './icon-close.svg?component';
  import IconError from './icon-error.svg?component';
  import IconInfo from './icon-info.svg?component';
  import IconSuccess from './icon-success.svg?component';
  import IconWarning from './icon-warning.svg?component';

  interface Props {
    notification: Notification;
  }

  let { notification }: Props = $props();

  let notificationsContext = getNotifications();

  let paused = $state(false);

  function handleMouseEnter() {
    if (!notification.autoClear) return;

    paused = true;
    notificationsContext.pauseAutoClear(notification);
  }

  function handleMouseLeave() {
    if (!notification.autoClear || notification.dismiss) return;

    paused = false;
    notificationsContext.setupAutoClear(notification);
  }

  function handleClose() {
    notificationsContext.removeNotification(notification);
  }
</script>

<div
  class="notification notification--{notification.type}"
  class:notification--in={!notification.dismiss}
  data-test-notification-message={notification.type}
  role="alert"
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
>
  <div class="notification__icon">
    {#if notification.type === 'info'}
      <IconInfo aria-hidden="true" width="16" height="16" />
    {:else if notification.type === 'success'}
      <IconSuccess aria-hidden="true" width="16" height="16" />
    {:else if notification.type === 'warning'}
      <IconWarning aria-hidden="true" width="16" height="16" />
    {:else if notification.type === 'error'}
      <IconError aria-hidden="true" width="16" height="16" />
    {/if}
  </div>

  <div class="notification__content">
    {notification.message}
  </div>

  <button type="button" class="notification__close" onclick={handleClose} aria-label="Dismiss notification">
    <IconClose aria-hidden="true" width="16" height="16" />
  </button>

  {#if notification.autoClear && !notification.dismiss}
    <div
      class="notification__countdown"
      style:animation-duration="{notification.clearDuration}ms"
      style:animation-play-state={paused ? 'paused' : 'running'}
    ></div>
  {/if}
</div>

<style>
  .notification {
    display: flex;
    align-items: stretch;
    position: relative;
    overflow: hidden;
    border-radius: 3px;
    color: white;
    max-height: 800px;
    animation:
      notification-hide 250ms cubic-bezier(0.33859, -0.42, 1, -0.22),
      notification-shrink 250ms 250ms cubic-bezier(0.5, 0, 0, 1);
    animation-fill-mode: forwards;
    margin-bottom: 1rem;
  }

  .notification--in {
    animation: notification-show 180ms cubic-bezier(0.175, 0.885, 0.32, 1.27499);
  }

  .notification--info {
    background-color: #3ea2ff;
  }

  .notification--success {
    background-color: #64ce83;
  }

  .notification--warning {
    background-color: #ff7f48;
  }

  .notification--error {
    background-color: #e74c3c;
  }

  .notification__icon {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 0.5rem 0;
    flex: none;
    background-color: rgba(255, 255, 255, 0.2);
    width: 30px;
    color: rgba(255, 255, 255, 0.74);
  }

  .notification__content {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    word-break: break-word;
    line-height: 1.5;
  }

  .notification__content :global(a) {
    color: #fff;
    text-decoration: underline;
  }

  .notification__close {
    margin: 0.5rem;
    align-self: flex-start;
    opacity: 0.74;
    cursor: pointer;
    background: none;
    border: none;
    padding: 0.25rem;
    color: inherit;
  }

  .notification__close:hover,
  .notification__close:focus {
    opacity: 1;
  }

  .notification__countdown {
    position: absolute;
    bottom: 0;
    left: 0;
    background-color: rgba(255, 255, 255, 0.4);
    width: 100%;
    height: 4px;
    animation-name: notification-countdown;
    animation-timing-function: linear;
    animation-iteration-count: 1;
    animation-fill-mode: forwards;
  }

  @keyframes notification-show {
    0% {
      opacity: 0;
      transform: perspective(450px) translate(0, -30px) rotateX(90deg);
    }
    100% {
      opacity: 1;
      transform: perspective(450px) translate(0, 0) rotateX(0deg);
    }
  }

  @keyframes notification-hide {
    0% {
      opacity: 1;
      transform: scale(1);
    }
    100% {
      opacity: 0;
      transform: scale(0.8);
    }
  }

  @keyframes notification-shrink {
    0% {
      opacity: 0;
      max-height: 800px;
      transform: scale(0.8);
    }
    100% {
      opacity: 0;
      max-height: 0;
      transform: scale(0.8);
    }
  }

  @keyframes notification-countdown {
    0% {
      width: 100%;
    }
    100% {
      width: 0%;
    }
  }
</style>
