<script lang="ts">
  import type { Snippet } from 'svelte';

  import TooltipContainer from '$lib/components/TooltipContainer.svelte';
  import { setTooltipContext } from '$lib/tooltip.svelte';

  let { children }: { children: Snippet } = $props();

  let tooltipContainerId = $props.id();
  setTooltipContext({ containerId: tooltipContainerId });
</script>

{@render children()}

<TooltipContainer />
