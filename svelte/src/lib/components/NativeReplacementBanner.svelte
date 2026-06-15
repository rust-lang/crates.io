<script lang="ts">
  import type { NativeReplacement } from '$lib/data/native-replacements';

  import Alert from '$lib/components/Alert.svelte';
  import { renderSimpleMarkdown } from '$lib/utils/markdown';

  interface Props {
    replacement: NativeReplacement;
  }

  let { replacement }: Props = $props();
</script>

<Alert variant="tip" data-test-native-replacement-banner>
  <strong>You might not need this dependency.</strong>
  <div class="description">
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- escaped micromark output -->
    {@html renderSimpleMarkdown(replacement.description)}
  </div>
  <p>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- always an external std/release-notes URL -->
    <a href={replacement.url} target="_blank" rel="noopener noreferrer">Learn more</a>
  </p>
</Alert>

<style>
  strong {
    font-size: 0.9em;
    font-weight: 500;
  }

  .description :global(p),
  p {
    margin: var(--space-2xs) 0 0;
    font-size: 0.9em;
    line-height: 1.4;
  }

  .description :global(code) {
    font-family: var(--font-monospace);
    font-size: 0.9em;
    letter-spacing: -2%;
    background: rgba(64, 186, 80, 0.25);
    padding: 0.1em 0.25em;
    border-radius: 0.3em;
  }
</style>
