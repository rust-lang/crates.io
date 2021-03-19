'use strict';

module.exports = {
  extends: ['octane', 'a11y'],

  pending: [{ moduleId: 'app/components/header', only: ['no-duplicate-landmark-elements'] }],
};
