import setupDeprecationWorkflow from 'ember-cli-deprecation-workflow';

setupDeprecationWorkflow({
  workflow: [
    {
      handler: 'silence',
      matchId: 'importing-inject-from-ember-service',
    },
    {
      handler: 'silence',
      matchId: 'warp-drive.deprecate-tracking-package',
    },
    {
      handler: 'silence',
      matchId: 'warp-drive:deprecate-legacy-request-methods',
    },
  ],
});
