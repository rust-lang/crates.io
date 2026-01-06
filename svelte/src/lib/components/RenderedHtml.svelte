<script lang="ts">
  import type { ResolvedScheme } from '$lib/color-scheme.svelte';

  import { highlightSyntax } from '$lib/attachments/highlight';
  import { renderMermaids } from '$lib/attachments/mermaid';
  import { getColorScheme } from '$lib/color-scheme.svelte';
  import TextContent from '$lib/components/TextContent.svelte';
  import { getNotifications } from '$lib/notifications.svelte';

  interface Props {
    html: string;
  }

  let { html }: Props = $props();

  let colorScheme = getColorScheme();
  let notifications = getNotifications();

  /**
   * Updates <source> media attributes in <picture> elements based on
   * the user's color scheme preference.
   *
   * Adapted from https://larsmagnus.co/blog/how-to-make-images-react-to-light-and-dark-mode
   */
  function updateSourceMedia(colorPreference: ResolvedScheme, html: string) {
    return (element: Element) => {
      // Ensure that the attachment is re-run when `html` changes
      void html;

      let pictures = element.querySelectorAll('picture');

      for (let picture of pictures) {
        let sources = picture.querySelectorAll<HTMLSourceElement>(
          'source[media*="prefers-color-scheme"], source[data-media*="prefers-color-scheme"]',
        );

        for (let source of sources) {
          // Preserve the source `media` as a data-attribute
          // to be able to switch between preferences
          if (source.media?.includes('prefers-color-scheme')) {
            source.dataset.media = source.media;
          }

          // If the source element `media` target is the `preference`,
          // override it to 'all' to show, or set it to 'none' to hide
          if (source.dataset.media?.includes(colorPreference)) {
            source.media = 'all';
          } else {
            source.media = 'none';
          }
        }
      }
    };
  }
</script>

<!--
  This component renders raw HTML. Be very careful with this since it
  can enable cross-site scripting attacks!
-->
<div
  {@attach highlightSyntax(html, 'pre > code:not(.language-mermaid)')}
  {@attach updateSourceMedia(colorScheme.resolvedScheme, html)}
  {@attach renderMermaids(html, notifications)}
>
  <TextContent>
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html html}
  </TextContent>
</div>
