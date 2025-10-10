import { setApplication } from '@ember/test-helpers';
import { start } from 'ember-qunit';
import * as QUnit from 'qunit';

import { loadTests } from 'ember-qunit/test-loader';
import { setup } from 'qunit-dom';

import Application from '../app';
import config from '../config/environment';
import registerMatchJsonAssertion from './helpers/match-json';
import { registerQUnitCallbacks } from './helpers/setup-msw';

setup(QUnit.assert);
registerMatchJsonAssertion(QUnit.assert);
registerQUnitCallbacks(QUnit);

setApplication(Application.create(config.APP));

loadTests();
start();
