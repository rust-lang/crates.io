<script lang="ts">
  import type { Unmaintained } from '$lib/utils/rustsec';

  import Alert from '$lib/components/Alert.svelte';

  interface Props {
    unmaintained: Unmaintained;
  }

  let { unmaintained }: Props = $props();
</script>

<Alert variant="warning" data-test-unmaintained-banner>
  <strong>This crate has been marked as unmaintained.</strong>
  <p>
    It is unlikely to receive further updates or fixes. See
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- always an external rustsec.org URL -->
    <a href={unmaintained.url} target="_blank" rel="noopener noreferrer">{unmaintained.id}</a>
    for details and possible alternatives.
  </p>
</Alert>

<style>
  strong {
    font-size: 0.9em;
    font-weight: 500;
  }

  p {
    margin: var(--space-2xs) 0 0;
    font-size: 0.9em;
    line-height: 1.4;
  }

  a {
    color: inherit;
    text-decoration: underline;
  }
</style>
