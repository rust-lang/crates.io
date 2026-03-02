<script module lang="ts">
  import { createContext } from 'svelte';

  export class TokenPageState {
    pendingToken: { id: number; token: string } | null = $state(null);
  }

  export const [getTokenPageState, setTokenPageState] = createContext<TokenPageState>();
</script>

<script lang="ts">
  let { children } = $props();

  setTokenPageState(new TokenPageState());
</script>

{@render children()}
