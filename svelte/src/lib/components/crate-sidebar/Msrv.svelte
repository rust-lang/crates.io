<script lang="ts">
  import Tooltip from '$lib/components/Tooltip.svelte';

  interface Props {
    msrv: string;
    edition?: string | null;
  }

  let { msrv, edition }: Props = $props();
</script>

<span>
  v{msrv}

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
