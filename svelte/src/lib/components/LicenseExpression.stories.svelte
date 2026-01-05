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

<Story name="MIT" args={{ license: 'MIT' }} />

<Story name="MIT OR Apache-2.0" args={{ license: 'MIT OR Apache-2.0' }} />

<Story name="MIT/Apache-2.0 (slash syntax)" args={{ license: 'MIT/Apache-2.0' }} />

<Story name="Complex Expression" args={{ license: '(Apache-2.0 OR MIT) AND BSD-3-Clause' }} />

<Story name="With Exception" args={{ license: 'GPL-2.0-or-later WITH Bison-exception-2.2' }} />

<Story name="Unknown License" args={{ license: 'UNKNOWN-LICENSE' }} />
