<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CrateReportForm from './CrateReportForm.svelte';

  const { Story } = defineMeta({
    title: 'support/CrateReportForm',
    component: CrateReportForm,
    tags: ['autodocs'],
    argTypes: {
      crate: {
        control: 'text',
      },
    },
  });
</script>

<Story name="Default" />
