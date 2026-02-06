<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import LicenseExpression from './LicenseExpression.svelte';

  const { Story } = defineMeta({
    title: 'LicenseExpression',
    component: LicenseExpression,
    tags: ['autodocs'],
    argTypes: {
      license: {
        control: 'text',
      },
    },
  });
</script>

<Story name="Default" args={{ license: 'MIT OR Apache-2.0' }} parameters={{ chromatic: { disableSnapshot: true } }} />

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>MIT</h1>
  <LicenseExpression license="MIT" />

  <h1>MIT OR Apache-2.0</h1>
  <LicenseExpression license="MIT OR Apache-2.0" />

  <h1>MIT/Apache-2.0 (slash syntax)</h1>
  <LicenseExpression license="MIT/Apache-2.0" />

  <h1>Complex Expression</h1>
  <LicenseExpression license="(Apache-2.0 OR MIT) AND BSD-3-Clause" />

  <h1>With Exception</h1>
  <LicenseExpression license="GPL-2.0-or-later WITH Bison-exception-2.2" />

  <h1>Unknown License</h1>
  <LicenseExpression license="UNKNOWN-LICENSE" />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
