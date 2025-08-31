import { setApplication } from '@ember/test-helpers';
import { start as startEmberExam } from 'ember-exam/test-support';
import { setupEmberOnerrorValidation } from 'ember-qunit';
import * as QUnit from 'qunit';

import { forceModulesToBeLoaded, sendCoverage } from 'ember-cli-code-coverage/test-support';
import { setup } from 'qunit-dom';

import Application from 'crates-io/app';
import config from 'crates-io/config/environment';

import registerMatchJsonAssertion from './helpers/match-json';

export async function start({ availableModules }) {
  setApplication(Application.create(config.APP));

  setup(QUnit.assert);
  registerMatchJsonAssertion(QUnit.assert);
  setupEmberOnerrorValidation();

  QUnit.done(async function () {
    forceModulesToBeLoaded();
    await sendCoverage();
  });

  await startEmberExam({ availableModules });
}
