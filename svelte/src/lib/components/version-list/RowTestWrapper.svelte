<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { createClient } from '@crates-io/api-client';

  import TooltipContainer from '$lib/components/TooltipContainer.svelte';
  import { setTooltipContext } from '$lib/tooltip.svelte';
  import { SessionState, setSession } from '$lib/utils/session.svelte';
  import Row from './Row.svelte';

  type Version = components['schemas']['Version'];

  interface Props {
    version: Version;
    crateName: string;
  }

  let { version, crateName }: Props = $props();
  let propsId = $props.id();

  // Row uses PrivilegedAction, which requires a session context.
  let session = new SessionState(createClient({ fetch }));
  setSession(session);

  // Row uses Tooltip, which requires a tooltip context.
  setTooltipContext({ containerId: `tooltip-container-${propsId}` });
</script>

<Row {version} {crateName} />
<TooltipContainer />
