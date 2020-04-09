import { setApplication } from '@ember/test-helpers';
import start from 'ember-exam/test-support/start';

import Application from '../app';
import config from '../config/environment';

setApplication(Application.create(config.APP));

start();
