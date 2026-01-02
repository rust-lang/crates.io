<script lang="ts">
  import Tooltip from '$lib/components/Tooltip.svelte';

  interface Props {
    edition: string;
  }

  let { edition }: Props = $props();

  let editionMsrv = $derived.by(() => {
    if (edition === '2018') {
      return '1.31.0';
    } else if (edition === '2021') {
      return '1.56.0';
    } else if (edition === '2024') {
      return '1.85.0';
    }
  });
</script>

<span>
  {edition} edition

  <Tooltip>
    This crate version does not declare a Minimum Supported Rust Version, but does require the {edition} Rust Edition.

    {#if editionMsrv}
      <div class="edition-msrv">
        {editionMsrv} was the first version of Rust in this edition, but this crate may require features that were added in
        later versions of Rust.
      </div>
    {/if}
  </Tooltip>
</span>

<style>
  .edition-msrv {
    margin-top: var(--space-2xs);
  }
</style>
