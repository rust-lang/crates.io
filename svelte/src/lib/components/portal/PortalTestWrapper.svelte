<script lang="ts">
  import { setContext } from 'svelte';

  import Portal from './Portal.svelte';
  import PortalTestContextReader from './PortalTestContextReader.svelte';

  interface Props {
    label: string;
  }

  let { label }: Props = $props();

  setContext('portal-test', 'context value');
</script>

<div data-test-parent>
  <Portal>
    <p data-test-label>{label}</p>
    <PortalTestContextReader />
  </Portal>
</div>
