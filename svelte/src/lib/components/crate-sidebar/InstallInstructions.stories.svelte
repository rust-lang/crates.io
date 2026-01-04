<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import InstallInstructions from './InstallInstructions.svelte';

  const { Story } = defineMeta({
    title: 'crate-sidebar/InstallInstructions',
    component: InstallInstructions,
    tags: ['autodocs'],
  });
</script>

<Story name="Library Only" args={{ crate: 'serde', version: '1.0.200', hasLib: true }} />

<Story
  name="Library with Exact Version"
  args={{ crate: 'serde', version: '1.0.200', exactVersion: true, hasLib: true }}
/>

<Story name="Single Binary" args={{ crate: 'ripgrep', version: '14.1.1', binNames: ['rg'] }} />

<Story name="Two Binaries" args={{ crate: 'cargo-edit', version: '0.12.3', binNames: ['cargo-add', 'cargo-rm'] }} />

<Story
  name="Multiple Binaries"
  args={{ crate: 'my-tool', version: '1.0.0', binNames: ['tool-a', 'tool-b', 'tool-c', 'tool-d'] }}
/>

<Story
  name="Binary and Library"
  args={{ crate: 'tokio-console', version: '0.1.12', binNames: ['tokio-console'], hasLib: true }}
/>

<Story name="Version with Build Metadata" args={{ crate: 'rand', version: '0.8.5+build.123', hasLib: true }} />
