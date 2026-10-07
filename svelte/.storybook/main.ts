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
  // Storybook removes the SvelteKit plugin that defines this constant, which
  // makes `$app/paths` throw in `storybook build` with SvelteKit 3.
  // See https://github.com/storybookjs/storybook/pull/36610.
  viteFinal: config => ({ ...config, define: { ...config.define, __SVELTEKIT_PAYLOAD__: '{}' } }),
};
export default config;
