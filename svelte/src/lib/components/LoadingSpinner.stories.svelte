<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import LoadingSpinner from './LoadingSpinner.svelte';

  const { Story } = defineMeta({
    title: 'LoadingSpinner',
    component: LoadingSpinner,
    tags: ['autodocs'],
    argTypes: {
      theme: {
        control: 'inline-radio',
        options: [undefined, 'light'],
      },
    },
  });
</script>

<Story name="Default" />

<Story name="Light Theme" args={{ theme: 'light' }} />

<Story name="Custom Size" asChild>
  <LoadingSpinner style="--spinner-size: 32px;" />
</Story>
