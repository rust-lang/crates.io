<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import NotificationStory from './NotificationStory.svelte';

  const { Story } = defineMeta({
    title: 'Notifications',
    component: NotificationStory,
  });
</script>

<Story name="Default" asChild>
  <NotificationStory>
    {#snippet children(notifications)}
      <div style="display: flex; flex-direction: column; align-items: start; gap: 8px; flex-wrap: wrap;">
        <button type="button" onclick={() => notifications.info('Info message')}>Info</button>
        <button type="button" onclick={() => notifications.success('Success message')}>Success</button>
        <button type="button" onclick={() => notifications.warning('Warning message')}>Warning</button>
        <button type="button" onclick={() => notifications.error('Error message')}>Error</button>
        <button
          type="button"
          onclick={() =>
            notifications.warning(
              'This is a much longer notification message that demonstrates how the component handles text wrapping and maintains readability even with extended content.',
            )}
        >
          Show Long Message
        </button>
        <button
          type="button"
          onclick={() => notifications.success('This will disappear in 3 seconds.', { clearDuration: 3000 })}
        >
          Show Quick (3s)
        </button>
        <button
          type="button"
          onclick={() => notifications.info('This message will stay until dismissed.', { autoClear: false })}
        >
          Show Persistent
        </button>
        <button type="button" onclick={() => notifications.clearAll()}>Clear All</button>
      </div>
    {/snippet}
  </NotificationStory>
</Story>
