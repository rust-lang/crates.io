<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { PlaygroundCrate } from '$lib/utils/playground';

  import { NotificationsState, setNotifications } from '$lib/notifications.svelte';
  import CrateSidebar from './CrateSidebar.svelte';

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Category = components['schemas']['Category'];
  type Owner = components['schemas']['Owner'];

  interface Props {
    crate: Crate;
    version: Version;
    categories?: Category[];
    owners: Owner[];
    requestedVersion?: boolean;
    playgroundCratesPromise: Promise<PlaygroundCrate[]>;
  }

  let { crate, version, categories = [], owners, playgroundCratesPromise, requestedVersion = false }: Props = $props();

  let notifications = new NotificationsState();
  setNotifications(notifications);
</script>

<CrateSidebar {crate} {version} {categories} {owners} {playgroundCratesPromise} {requestedVersion} />
