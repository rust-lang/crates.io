<script lang="ts">
  import { setTooltipContext } from '$lib/tooltip.svelte';
  import Tooltip from './Tooltip.svelte';
  import TooltipContainer from './TooltipContainer.svelte';

  interface Props {
    text: string;
    width: string;
    delay?: number;
    onlyWhenTruncated?: boolean;
  }

  let { text, width, delay = 0, onlyWhenTruncated = false }: Props = $props();
  let propsId = $props.id();

  setTooltipContext({ containerId: `tooltip-container-${propsId}` });
</script>

<div data-test-anchor style:width style:overflow="hidden" style:white-space="nowrap" style:text-overflow="ellipsis">
  {text}
  <Tooltip {text} {delay} {onlyWhenTruncated} />
</div>

<TooltipContainer />
