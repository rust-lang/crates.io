import Service, { inject as service } from '@ember/service';
import { htmlSafe } from '@ember/string';
import { tracked } from '@glimmer/tracking';

import { rawTimeout, task } from 'ember-concurrency';
import { buildWaiter } from 'ember-test-waiters';

const SPEED = 200;

let waiter = buildWaiter('progress-bar');

export default class ProgressService extends Service {
  @service router;

  @tracked _style = '';

  get style() {
    return htmlSafe(this._style);
  }

  handle(thenable) {
    this.counterTask.perform(thenable);
  }

  @task(function* (promise) {
    this.updateTask.perform();
    yield promise;
  })
  counterTask;

  @(task(function* () {
    let token = waiter.beginAsync();

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

    waiter.endAsync(token);
  }).drop())
  updateTask;
}
