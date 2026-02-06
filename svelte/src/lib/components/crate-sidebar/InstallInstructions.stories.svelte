<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import InstallInstructions from './InstallInstructions.svelte';

  const { Story } = defineMeta({
    title: 'crate-sidebar/InstallInstructions',
    component: InstallInstructions,
    tags: ['autodocs'],
  });
</script>

<Story
  name="Default"
  args={{ crate: 'serde', version: '1.0.200', hasLib: true, exactVersion: false, binNames: [] }}
  parameters={{ chromatic: { disableSnapshot: true } }}
/>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Library Only</h1>
  <InstallInstructions crate="serde" version="1.0.200" hasLib={true} />

  <h1>Library with Exact Version</h1>
  <InstallInstructions crate="serde" version="1.0.200" exactVersion={true} hasLib={true} />

  <h1>Single Binary</h1>
  <InstallInstructions crate="ripgrep" version="14.1.1" binNames={['rg']} />

  <h1>Two Binaries</h1>
  <InstallInstructions crate="cargo-edit" version="0.12.3" binNames={['cargo-add', 'cargo-rm']} />

  <h1>Multiple Binaries</h1>
  <InstallInstructions crate="my-tool" version="1.0.0" binNames={['tool-a', 'tool-b', 'tool-c', 'tool-d']} />

  <h1>Binary and Library</h1>
  <InstallInstructions crate="tokio-console" version="0.1.12" binNames={['tokio-console']} hasLib={true} />

  <h1>Version with Build Metadata</h1>
  <InstallInstructions crate="rand" version="0.8.5+build.123" hasLib={true} />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;

    &:not(:first-child) {
      margin-top: 3rem;
    }
  }
</style>
