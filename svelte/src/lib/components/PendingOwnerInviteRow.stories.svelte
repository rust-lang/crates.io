<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import NotificationDecorator from '$lib/storybook/NotificationDecorator.svelte';
  import PendingOwnerInviteRow from './PendingOwnerInviteRow.svelte';

  const { Story } = defineMeta({
    title: 'PendingOwnerInviteRow',
    component: PendingOwnerInviteRow,
  });

  const INVITE = {
    crate_id: 1,
    crate_name: 'serde',
    inviter_login: 'dtolnay',
    created_at: '2024-12-24T12:34:56Z',
  };
</script>

<Story name="Default" asChild>
  <NotificationDecorator>
    <PendingOwnerInviteRow invite={INVITE} />
  </NotificationDecorator>
</Story>
