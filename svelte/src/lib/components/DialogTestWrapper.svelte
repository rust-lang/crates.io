<script lang="ts">
  import Dialog from './Dialog.svelte';

  interface Props {
    fullscreen?: boolean;
    /** Links the trigger to the dialog as its invoker instead of using a click handler. */
    useInvoker?: boolean;
  }

  let { fullscreen, useInvoker = false }: Props = $props();

  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
</script>

<button type="button" data-test-trigger bind:this={trigger} onclick={useInvoker ? undefined : () => (open = true)}>
  Open
</button>

<h1 data-test-heading>
  <Dialog bind:open title="Example dialog" {fullscreen} invoker={useInvoker ? trigger : undefined}>
    <p data-test-content>Dialog content</p>
  </Dialog>
</h1>
