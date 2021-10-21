import Service, { inject as service } from '@ember/service';
import { htmlSafe } from '@ember/template';
import { tracked } from '@glimmer/tracking';
import Ember from 'ember';

import { didCancel, dropTask, rawTimeout, task } from 'ember-concurrency';

const SPEED = 200;

export default class ProgressService extends Service {
  @service router;
  @service sentry;

  @tracked _style = '';

  get style() {
    return htmlSafe(this._style);
  }

  handle(thenable) {
    this.counterTask.perform(thenable).catch(() => {
      // the `counterTask` performs the passed in `thenable` (aka. `Promise`), which might be rejected
      // and cause the task to fail too. since we don't care about the failure here, we'll just ignore it.
    });
  }

  @task *counterTask(promise) {
    this.updateTask.perform().catch(error => {
      if (!didCancel(error)) {
        // this task shouldn't be able to fail, but if it does we'll let Sentry know
        this.sentry.captureException(error);
      }
    });

    yield promise;
  }

  @dropTask *updateTask() {
    if (Ember.testing) return;

    let progress = 0;
    this._style = `width: 0%`;

    while (this.counterTask.isRunning) {
      yield rawTimeout(SPEED);

      let currentAmount;
      if (progress >= 0 && progress < 0.2) {
        currentAmount = 0.1;
      } else if (progress >= 0.2 && progress < 0.5) {
        currentAmount = 0.04;
      } else if (progress >= 0.5 && progress < 0.8) {
        currentAmount = 0.02;
      } else if (progress >= 0.8 && progress < 0.99) {
        currentAmount = 0.005;
      } else {
        currentAmount = 0;
      }

      progress += currentAmount;
      if (progress > 0.998) {
        progress = 0.998;
      }

      this._style = `transition: width ${SPEED}ms linear; width: ${progress * 100}%`;
    }

    this._style = `transition: width ${SPEED}ms linear; width: 100%`;
    yield rawTimeout(SPEED);
    this._style = `transition: opacity ${SPEED * 2}ms linear; width: 100%; opacity: 0`;
  }
}
