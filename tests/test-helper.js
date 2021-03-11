import { setApplication } from '@ember/test-helpers';
import start from 'ember-exam/test-support/start';
import * as QUnit from 'qunit';

import { setup } from 'qunit-dom';

import Application from '../app';
import config from '../config/environment';
import registerMatchJsonAssertion from './helpers/match-json';

setup(QUnit.assert);
registerMatchJsonAssertion(QUnit.assert);

setApplication(Application.create(config.APP));

start();
