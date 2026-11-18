<script lang="ts">
  import { setTooltipContext } from '#lib/tooltip.svelte.ts';
  import TooltipContainer from './TooltipContainer.svelte';
  import UsernameMismatchNote from './UsernameMismatchNote.svelte';

  interface Props {
    name: string;
    login: string;
  }

  let { name, login }: Props = $props();
  let propsId = $props.id();

  setTooltipContext({ containerId: `tooltip-container-${propsId}` });
</script>

<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
<a href="/users/{login}">{name} <UsernameMismatchNote {login} /></a>

<TooltipContainer />
