<script lang="ts">
  import { getTooltipContext } from '#lib/tooltip.svelte.ts';

  let tooltipContext = getTooltipContext();
</script>

<div id={tooltipContext.containerId}></div>
