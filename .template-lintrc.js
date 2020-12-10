'use strict';

module.exports = {
  extends: ['octane', 'a11y'],

  pending: [
    { moduleId: 'app/components/email-input', only: ['require-input-label'] },
    { moduleId: 'app/components/header', only: ['no-duplicate-landmark-elements'] },
    { moduleId: 'app/templates/catch-all', only: ['require-input-label'] },
    { moduleId: 'app/components/settings/api-tokens', only: ['require-input-label'] },
  ],
};
