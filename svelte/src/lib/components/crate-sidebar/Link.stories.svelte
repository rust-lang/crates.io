<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Link from './Link.svelte';

  const { Story } = defineMeta({
    title: 'crate-sidebar/Link',
    component: Link,
    tags: ['autodocs'],
  });
</script>

<Story name="Generic Link" args={{ title: 'Homepage', url: 'https://www.rust-lang.org' }} />
<Story name="GitHub Link" args={{ title: 'Repository', url: 'https://github.com/rust-lang/crates.io' }} />
<Story name="GitLab Link" args={{ title: 'Repository', url: 'https://gitlab.com/example/project' }} />
<Story name="Codeberg Link" args={{ title: 'Repository', url: 'https://codeberg.org/example/project' }} />
<Story name="docs.rs Link" args={{ title: 'Documentation', url: 'https://docs.rs/tracing' }} />
