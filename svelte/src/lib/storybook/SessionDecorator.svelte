<script lang="ts">
  import type { AuthenticatedUser } from '$lib/utils/session.svelte';
  import type { Snippet } from 'svelte';

  import { createClient } from '@crates-io/api-client';

  import { SessionState, setSession } from '$lib/utils/session.svelte';

  let { children, user }: { children: Snippet; user?: AuthenticatedUser } = $props();

  let session = new SessionState(createClient({ fetch }));
  setSession(session);

  // svelte-ignore state_referenced_locally
  session.setUser(user ?? null);
</script>

{@render children()}
