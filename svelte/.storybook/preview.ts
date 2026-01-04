import type { Preview } from '@storybook/sveltekit';

import { themes } from 'storybook/theming';

import '../src/lib/css/global.css';

import ColorSchemeDecorator from '../src/lib/storybook/ColorSchemeDecorator.svelte';
import HeaderSearchDecorator from '../src/lib/storybook/HeaderSearchDecorator.svelte';
import NotificationDecorator from '../src/lib/storybook/NotificationDecorator.svelte';
import TooltipDecorator from '../src/lib/storybook/TooltipDecorator.svelte';

const preview: Preview = {
  decorators: [
    () => ColorSchemeDecorator,
    () => HeaderSearchDecorator,
    () => NotificationDecorator,
    () => TooltipDecorator,
  ],
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
    docs: {
      // Storybook's docs pages default to light theme regardless of system preference.
      // The `normal` theme adapts to system preference, so we set it here to keep things consistent.
      theme: themes.normal,
    },
  },
};

export default preview;
