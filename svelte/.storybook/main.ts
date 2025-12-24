import type { StorybookConfig } from '@storybook/sveltekit';

const config: StorybookConfig = {
  core: {
    disableTelemetry: true,
  },
  framework: '@storybook/sveltekit',
  stories: ['../src/**/*.stories.@(js|ts|svelte)'],
  addons: [
    '@storybook/addon-svelte-csf',
    '@chromatic-com/storybook',
    '@storybook/addon-vitest',
    '@storybook/addon-a11y',
    '@storybook/addon-docs',
  ],
};
export default config;
