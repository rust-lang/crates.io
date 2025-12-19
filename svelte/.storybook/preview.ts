import type { Preview } from '@storybook/sveltekit';

import '../src/lib/css/global.css';

const preview: Preview = {
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
