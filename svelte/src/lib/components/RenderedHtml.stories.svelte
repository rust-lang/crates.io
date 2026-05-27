<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { loadMermaid } from '$lib/attachments/mermaid';
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

    <h2>Alerts</h2>
    <div class="markdown-alert markdown-alert-note">
      <p class="markdown-alert-title">Note</p>
      <p>Useful information that users should know, even when skimming content.</p>
    </div>
    <div class="markdown-alert markdown-alert-tip">
      <p class="markdown-alert-title">Tip</p>
      <p>Helpful advice for doing things better or more easily.</p>
    </div>
    <div class="markdown-alert markdown-alert-important">
      <p class="markdown-alert-title">Important</p>
      <p>Key information users need to know to achieve their goal.</p>
    </div>
    <div class="markdown-alert markdown-alert-warning">
      <p class="markdown-alert-title">Warning</p>
      <p>Urgent info that needs immediate user attention to avoid problems.</p>
    </div>
    <div class="markdown-alert markdown-alert-caution">
      <p class="markdown-alert-title">Caution</p>
      <p>Advises about risks or negative outcomes of certain actions.</p>
    </div>

    <div class="markdown-alert markdown-alert-note">
      <p class="markdown-alert-title">Note</p>
      <div class="markdown-alert markdown-alert-important">
        <p class="markdown-alert-title">Important</p>
        <div class="markdown-alert markdown-alert-caution">
          <p class="markdown-alert-title">Caution</p>
          <p>Rick roll</p>
          <p>Never gonna give you up</p>
        </div>
      </div>
    </div>
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
    <RenderedHtml html={args.html} />
  </div>
{/snippet}

<Story name="Default" args={{ html: EXAMPLE_HTML }} {template} />

<Story name="With Mermaid" args={{ html: MERMAID_HTML }} loaders={[() => loadMermaid()]} {template} />

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
