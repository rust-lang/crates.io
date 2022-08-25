/* eslint-env browser */

self.deprecationWorkflow = self.deprecationWorkflow || {};
self.deprecationWorkflow.config = {
  workflow: [
    // disabled because it's a false positive caused by ember-concurrency
    // checking if `__ec_cancel__` is available.
    { handler: 'silence', matchId: 'ember-data:model-save-promise' },
  ],
};
