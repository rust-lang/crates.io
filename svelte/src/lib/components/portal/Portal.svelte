<!--
  @component
  Renders its children at the end of `<body>`, keeping the contexts of the
  surrounding component tree.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { getAllContexts, mount, unmount } from 'svelte';

  import PortalContent from './PortalContent.svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  let context = getAllContexts();

  // Moving rendered nodes would break Svelte's DOM bookkeeping, so the
  // children are mounted as a separate component instead.
  $effect(() => {
    let content = mount(PortalContent, { target: document.body, props: { children }, context });
    return () => unmount(content);
  });
</script>
