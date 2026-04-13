<script lang="ts">
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { normalizeMsrv } from '$lib/utils/msrv';

  interface Props {
    msrv: string;
    edition?: string | null;
  }

  let { msrv, edition }: Props = $props();

  let displayMsrv = $derived(normalizeMsrv(msrv));
</script>

<span>
  v{displayMsrv}

  <Tooltip>
    "Minimum Supported Rust Version"

    {#if edition}
      <div class="edition">requires Rust Edition {edition}</div>
    {/if}
  </Tooltip>
</span>

<style>
  .edition {
    margin-top: var(--space-2xs);
  }
</style>
