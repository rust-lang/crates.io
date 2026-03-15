<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import WorkflowVerification from './WorkflowVerification.svelte';

  const { Story } = defineMeta({
    title: 'WorkflowVerification',
    component: WorkflowVerification,
    tags: ['autodocs'],
  });
</script>

<Story name="Initial" args={{ url: '' }} parameters={{ chromatic: { disableSnapshot: true } }} />

<Story
  name="Success"
  args={{ url: 'https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/ci.yml' }}
  parameters={{ chromatic: { disableSnapshot: true } }}
/>

<Story
  name="Not Found"
  args={{ url: 'https://raw.githubusercontent.com/rust-lang/crates.io/HEAD/.github/workflows/missing.yml' }}
  parameters={{ chromatic: { disableSnapshot: true } }}
/>
