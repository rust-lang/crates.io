<script lang="ts">
  import { getTooltipContext } from '$lib/tooltip.svelte';

  let tooltipContext = getTooltipContext();
</script>

<div id={tooltipContext.containerId}></div>
