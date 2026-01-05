<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ColorSchemeMenu from './ColorSchemeMenu.svelte';
  import RenderedHtml from './RenderedHtml.svelte';

  const { Story } = defineMeta({
    title: 'RenderedHtml',
    component: RenderedHtml,
    tags: ['autodocs'],
  });

  const EXAMPLE_HTML = `
    <h2>Example README</h2>
    <p>This is a paragraph with some <code>inline code</code> in it.</p>
    <pre><code class="language-rust">fn main() {
    println!("Hello, world!");
}</code></pre>
    <p>Here's another code block without a language:</p>
    <pre><code>some generic code</code></pre>
  `;

  const MERMAID_HTML = `
    <h2>With Mermaid Diagram</h2>
    <p>Below is a mermaid diagram:</p>
    <pre><code class="language-mermaid">graph TD
    A[Start] --> B{Is it working?}
    B -->|Yes| C[Great!]
    B -->|No| D[Debug]
    D --> B</code></pre>
  `;

  const DARK_MODE_IMAGES_HTML = `
    <h2>Dark Mode Images</h2>
    <p>This image changes based on color scheme:</p>
    <picture>
      <source srcset="https://placehold.co/200x100/white/black?text=Light" media="(prefers-color-scheme: light)">
      <source srcset="https://placehold.co/200x100/black/white?text=Dark" media="(prefers-color-scheme: dark)">
      <img src="https://placehold.co/200x100/gray/white?text=Fallback" alt="Example">
    </picture>
  `;
</script>

{#snippet template(args: { html: string })}
  <div class="docs">
    <ColorSchemeMenu style="position: absolute; right: var(--space-l)" />
    <RenderedHtml html={args.html} />
  </div>
{/snippet}

<Story name="Default" args={{ html: EXAMPLE_HTML }} {template} />

<Story name="With Mermaid" args={{ html: MERMAID_HTML }} {template} />

<Story name="Dark Mode Images" args={{ html: DARK_MODE_IMAGES_HTML }} {template} />

<style>
  .docs {
    margin-bottom: var(--space-l);
    padding: var(--space-m) var(--space-l);
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);
  }
</style>
