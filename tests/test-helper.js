// TODO re-enable ember exam
import emberExamStart from 'ember-exam/test-support/start';
import { forceModulesToBeLoaded, sendCoverage } from 'ember-cli-code-coverage/test-support';
import registerMatchJsonAssertion from './helpers/match-json';


import Application from 'crates-io/app';
import config from 'crates-io/config/environment';
import * as QUnit from 'qunit';
import { setApplication } from '@ember/test-helpers';
import { setup } from 'qunit-dom';
import { start as qunitStart, setupEmberOnerrorValidation } from 'ember-qunit';

export function start() {
  setApplication(Application.create(config.APP));

  setup(QUnit.assert);
  registerMatchJsonAssertion(QUnit.assert);
  setupEmberOnerrorValidation();

  QUnit.done(async function () {
    forceModulesToBeLoaded();
    await sendCoverage();
  });

  qunitStart();
}
