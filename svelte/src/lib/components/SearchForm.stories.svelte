<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import SearchForm from './SearchForm.svelte';

  const { Story } = defineMeta({
    title: 'SearchForm',
    component: SearchForm,
    tags: ['autodocs'],
    globals: {
      backgrounds: { value: 'header' },
    },
    argTypes: {
      size: {
        control: 'inline-radio',
        options: [undefined, 'big'],
      },
    },
  });
</script>

<Story name="Default" />

<Story name="Big" args={{ size: 'big' }} />

<Story name="with autofocus" args={{ autofocus: true }} />
