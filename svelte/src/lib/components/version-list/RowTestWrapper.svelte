<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { createClient } from '@crates-io/api-client';

  import { SessionState, setSession } from '#lib/utils/session.svelte.ts';
  import Row from './Row.svelte';

  type Version = components['schemas']['Version'];

  interface Props {
    version: Version;
    crateName: string;
    isHighestOfReleaseTrack?: boolean;
  }

  let { version, crateName, isHighestOfReleaseTrack }: Props = $props();

  // Row uses PrivilegedAction, which requires a session context.
  let session = new SessionState(createClient({ fetch }));
  setSession(session);
</script>

<Row {version} {crateName} {isHighestOfReleaseTrack} />
