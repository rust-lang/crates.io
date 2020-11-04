'use strict';

module.exports = {
  extends: ['octane', 'a11y'],

  pending: [
    { moduleId: 'app/components/email-input', only: ['require-input-label'] },
    { moduleId: 'app/components/header', only: ['no-duplicate-landmark-elements'] },
    // see https://github.com/ember-template-lint/ember-template-lint/issues/1604
    { moduleId: 'app/components/pagination', only: ['no-invalid-link-title'] },
    { moduleId: 'app/templates/catch-all', only: ['require-input-label'] },
    { moduleId: 'app/components/settings/api-tokens', only: ['require-input-label'] },
  ],
};
