<script lang="ts">
  import AccountChip from './AccountChip.svelte';

  interface Props {
    handle: string;
    href: string;
    mismatched?: boolean;
    width?: string;
  }

  let { handle, href, mismatched = false, width = 'max-content' }: Props = $props();
</script>

<div style:width>
  <AccountChip provider="github" {handle} {href} {mismatched} />
</div>
