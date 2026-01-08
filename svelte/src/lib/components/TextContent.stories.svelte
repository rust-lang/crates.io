<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import TextContent from './TextContent.svelte';

  const { Story } = defineMeta({
    title: 'TextContent',
    component: TextContent,
    tags: ['autodocs'],
  });
</script>

<Story name="Default">
  <p>This is some example text content with a <code>code snippet</code> inside.</p>
  <pre>fn main() {'{'} println!("Hello, world!"); }</pre>
</Story>

<Story name="Boxed" args={{ boxed: true }}>
  <h2>Boxed Content</h2>
  <p>This content appears in a boxed container with padding and background color.</p>
</Story>

<Story name="Alerts">
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
</Story>
