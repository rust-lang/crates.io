import type { Preview } from '@storybook/sveltekit';

import '../src/lib/css/global.css';

import ColorSchemeDecorator from '../src/lib/storybook/ColorSchemeDecorator.svelte';
import HeaderSearchDecorator from '../src/lib/storybook/HeaderSearchDecorator.svelte';
import TooltipDecorator from '../src/lib/storybook/TooltipDecorator.svelte';

const preview: Preview = {
  decorators: [() => ColorSchemeDecorator, () => HeaderSearchDecorator, () => TooltipDecorator],
  initialGlobals: {
    backgrounds: { value: 'content' },
  },
  parameters: {
    backgrounds: {
      options: {
        content: { name: 'Dynamic', value: 'var(--main-bg)' },
        header: { name: 'Header', value: 'var(--header-bg-color)' },
      },
    },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
};

export default preview;
