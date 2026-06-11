<script lang="ts">
  import type { Unmaintained } from '$lib/utils/rustsec';

  import Alert from '$lib/components/Alert.svelte';

  interface Props {
    unmaintained: Unmaintained;
  }

  let { unmaintained }: Props = $props();
</script>

<Alert variant="warning" data-test-unmaintained-banner>
  <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- always an external rustsec.org URL -->
  <a href={unmaintained.url} target="_blank" rel="noopener noreferrer">{unmaintained.id}</a>
  marks this crate as unmaintained, so it is unlikely to receive further updates or fixes. Consider looking for an actively
  maintained alternative (some might be listed in the advisory).
</Alert>
