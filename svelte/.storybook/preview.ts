import type { Preview } from '@storybook/sveltekit';

import { themes } from 'storybook/theming';

import '../src/lib/css/global.css';

import ColorSchemeDecorator from '../src/lib/storybook/ColorSchemeDecorator.svelte';
import HeaderSearchDecorator from '../src/lib/storybook/HeaderSearchDecorator.svelte';
import NotificationDecorator from '../src/lib/storybook/NotificationDecorator.svelte';
import SessionDecorator from '../src/lib/storybook/SessionDecorator.svelte';
import TooltipDecorator from '../src/lib/storybook/TooltipDecorator.svelte';

const THEME_ITEMS = [
  { value: 'light', icon: 'sun' },
  { value: 'dark', icon: 'moon' },
  { value: 'system', icon: 'mirror' },
];

const preview: Preview = {
  decorators: [
    (_, context) => {
      let { theme } = context.globals;
      return {
        Component: ColorSchemeDecorator,
        props: {
          theme: theme === '_reset' ? 'system' : theme,
        },
      };
    },
    () => HeaderSearchDecorator,
    () => NotificationDecorator,
    () => SessionDecorator,
    () => TooltipDecorator,
  ],
  globalTypes: {
    theme: {
      description: 'Global theme for components',
      toolbar: {
        title: 'Theme',
        icon: 'mirror',
        items: THEME_ITEMS.map(c => ({
          ...c,
          title: c.value.charAt(0).toUpperCase() + c.value.slice(1),
        })),
        showName: true,
        dynamicTitle: true,
      },
    },
  },
  initialGlobals: {
    backgrounds: { value: 'content' },
    theme: { value: 'system' },
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
