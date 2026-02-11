<script lang="ts">
  import { getSession } from '$lib/utils/session.svelte';

  let { children, data } = $props();

  let session = getSession();

  // The root layout's `userPromise.then()` sets the user via a microtask,
  // which means `session.currentUser` is still `null` during the initial
  // synchronous render. Since the settings `+layout.ts` already awaited
  // the user, we can set it synchronously here to avoid a render flash
  // where child components briefly see `currentUser: null`.
  //
  // svelte-ignore state_referenced_locally
  session.setUser(data.user);
</script>

{@render children()}
