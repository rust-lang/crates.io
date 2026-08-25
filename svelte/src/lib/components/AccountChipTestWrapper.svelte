<script lang="ts">
  import { setTooltipContext } from '$lib/tooltip.svelte';
  import AccountChip from './AccountChip.svelte';
  import TooltipContainer from './TooltipContainer.svelte';

  interface Props {
    handle: string;
    href: string;
    mismatched?: boolean;
    width?: string;
  }

  let { handle, href, mismatched = false, width = 'max-content' }: Props = $props();
  let propsId = $props.id();

  setTooltipContext({ containerId: `tooltip-container-${propsId}` });
</script>

<div style:width>
  <AccountChip provider="github" {handle} {href} {mismatched} />
</div>

<TooltipContainer />
