import { action } from '@ember/object';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import Ember from 'ember';

import { didCancel, dropTask, rawTimeout, task } from 'ember-concurrency';

import ajax from '../utils/ajax';

export default class ApplicationRoute extends Route {
  @service notifications;
  @service progress;
  @service router;
  @service session;
  @service playground;
  @service sentry;

  async beforeModel(transition) {
    this.setSentryTransaction(transition);
    this.router.on('routeWillChange', transition => this.setSentryTransaction(transition));
    this.router.on('routeDidChange', transition => this.setSentryTransaction(transition));

    // trigger the task, but don't wait for the result here
    //
    // we don't need a `catch()` block here because network
    // errors are already dealt with inside of the task
    // and any other errors should end up on Sentry.
    //
    // eslint-disable-next-line ember-concurrency/no-perform-without-catch
    this.session.loadUserTask.perform();

    // trigger the preload task, but don't wait for the task to finish.
    this.preloadPlaygroundCratesTask.perform().catch(() => {
      // ignore all errors since we're only preloading here
    });

    this.checkReadOnlyStatusTask.perform().catch(error => {
      if (!didCancel(error) && !error.isServerError && !error.isNetworkError) {
        // send unexpected errors to Sentry, but don't bother the user for this optional feature
        this.sentry.captureException(error);
      }
    });

    // load ResizeObserver polyfill, only if required.
    if (!('ResizeObserver' in window)) {
      console.debug('Loading ResizeObserver polyfill…');
      let module = await import('@juggle/resize-observer');
      window.ResizeObserver = module.ResizeObserver;
    }
  }

  @action loading(transition) {
    this.progress.handle(transition);
    return true;
  }

  preloadPlaygroundCratesTask = task(async () => {
    await rawTimeout(1000);
    await this.playground.loadCrates();
  });

  checkReadOnlyStatusTask = dropTask(async () => {
    // delay the status check to let the more relevant data load first
    let timeout = Ember.testing ? 0 : 1000;
    await rawTimeout(timeout);

    let { read_only: readOnly } = await ajax('/api/v1/site_metadata');
    if (readOnly) {
      let message =
        'crates.io is currently in read-only mode for maintenance reasons. ' +
        'Some functionality will be temporarily unavailable.';

      this.notifications.info(message, { autoClear: false });
    }
  });

  setSentryTransaction(transition) {
    let name = transition.to?.name;
    if (name) {
      this.sentry.getCurrentScope().setTransactionName(name);
    }
  }
}
