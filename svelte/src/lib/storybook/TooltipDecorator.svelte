<script lang="ts">
  import type { Snippet } from 'svelte';

  import TooltipContainer from '$lib/components/TooltipContainer.svelte';
  import { setTooltipContext } from '$lib/tooltip.svelte';

  let { children }: { children: Snippet } = $props();
  let propsId = $props.id();

  setTooltipContext({ containerId: `tooltip-container-${propsId}` });
</script>

{@render children()}

<TooltipContainer />
