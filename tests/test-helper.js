import Application from '../app';
import { setApplication } from '@ember/test-helpers';
import { start } from 'ember-qunit';
import loadEmberExam from 'ember-exam/test-support/load';

setApplication(Application.create({ autoboot: false }));

loadEmberExam();
start();
