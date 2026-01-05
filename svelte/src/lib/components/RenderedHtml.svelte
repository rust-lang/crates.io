<script lang="ts">
  import TextContent from '$lib/components/TextContent.svelte';

  interface Props {
    html: string;
  }

  let { html }: Props = $props();

  // TODO: implement highlightSyntax attachment
  // - Use highlight.js to syntax highlight code blocks
  // - Select elements matching 'pre > code:not(.language-mermaid)'
  // - Re-run when `html` changes
  // - See: app/modifiers/highlight-syntax.js

  // TODO: implement updateSourceMedia attachment
  // - Update <source> media attributes in <picture> elements for dark/light mode
  // - React to `colorScheme.resolvedScheme` changes
  // - Re-run when `html` changes
  // - See: app/modifiers/update-source-media.js

  // TODO: implement renderMermaids attachment
  // - Render mermaid diagrams from .language-mermaid code blocks
  // - Requires lazy-loading mermaid.js library
  // - Re-run when `html` changes
  // - See: app/modifiers/render-mermaids.js and app/services/mermaid.js
</script>

<!--
  This component renders raw HTML. Be very careful with this since it
  can enable cross-site scripting attacks!
-->
<TextContent>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html}
</TextContent>
