<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CodeViewer from './CodeViewer.svelte';

  const { Story } = defineMeta({
    title: 'CodeViewer',
    component: CodeViewer,
    tags: ['autodocs'],
    argTypes: {
      colorScheme: { control: 'inline-radio', options: ['light', 'dark'] },
    },
    args: {
      colorScheme: 'light',
    },
  });

  const FILES = [
    {
      path: 'Cargo.toml',
      text: '[package]\nname = "example"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nserde = "1"\nserde_json = "1"\n',
      meta: '88 B',
    },
    {
      path: 'src/main.rs',
      text: 'fn main() {\n    println!("Hello, world!");\n}\n',
      meta: '42 B',
    },
    {
      path: 'README.md',
      text: '# Example\n\nA sample crate used in the CodeViewer story.\n',
      meta: '52 B',
    },
  ];
</script>

<script>
</script>

<Story name="Default">
  {#snippet template(args: { colorScheme: 'light' | 'dark' })}
    <div class="viewer">
      <CodeViewer content={FILES[0]} colorScheme={args.colorScheme} />
    </div>

    <div class="viewer">
      <CodeViewer content={FILES[1]} colorScheme={args.colorScheme} />
    </div>

    <div class="viewer">
      <CodeViewer content={FILES[2]} colorScheme={args.colorScheme} />
    </div>
  {/snippet}
</Story>

<style>
  .viewer {
    display: flex;
    height: 200px;
    margin-bottom: var(--space-m);
  }
</style>
