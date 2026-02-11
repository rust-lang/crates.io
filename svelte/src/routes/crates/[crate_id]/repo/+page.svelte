<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getNotifications } from '$lib/notifications.svelte';

  let notifications = getNotifications();

  notifications.error('Crate does not supply a repository URL');
  goto(resolve('/crates/[crate_id]', { crate_id: page.params.crate_id! }));
</script>
