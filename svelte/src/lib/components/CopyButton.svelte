<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  import { getNotifications } from '$lib/notifications.svelte';

  interface Props extends HTMLButtonAttributes {
    copyText: string;
    children: Snippet;
  }

  let { copyText, children, ...restProps }: Props = $props();

  let notifications = getNotifications();

  async function copy() {
    try {
      await navigator.clipboard.writeText(copyText);
      notifications.success('Copied to clipboard!');
    } catch {
      notifications.error('Copy to clipboard failed!');
    }
  }
</script>

<button type="button" onclick={copy} {...restProps}>
  {@render children()}
</button>
