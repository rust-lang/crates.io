import setupDeprecationWorkflow from 'ember-cli-deprecation-workflow';

/**
 * Docs: https://github.com/ember-cli/ember-cli-deprecation-workflow
 */
setupDeprecationWorkflow({
  /**
    false by default, but if a developer / team wants to be more aggressive about being proactive with
    handling their deprecations, this should be set to "true"
  */
  throwOnUnhandled: false,
  /* to generate this list, run your app for a while (or run the test suite),
   * and then run in the browser console:
   *
   *    deprecationWorkflow.flushDeprecations()
   *
   * And copy the handlers here
   */
  workflow: [
    {
      handler: 'silence',
      matchId: 'importing-inject-from-ember-service',
    },
    {
      handler: 'silence',
      matchId: 'ember-data:deprecate-legacy-imports',
    },
  ],
});
