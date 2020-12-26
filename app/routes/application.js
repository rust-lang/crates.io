import { action } from '@ember/object';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class ApplicationRoute extends Route {
  @service googleCharts;
  @service progress;
  @service session;

  beforeModel() {
    // trigger the task, but don't wait for the result here
    //
    // we don't need a `catch()` block here because network
    // errors are already dealt with inside of the task
    // and any other errors should end up on Sentry.
    //
    // eslint-disable-next-line ember-concurrency/no-perform-without-catch
    this.session.loadUserTask.perform();

    // start loading the Google Charts JS library already
    // and ignore any errors since we will catch them again
    // anyway when we call `load()` from the `DownloadGraph`
    // component
    this.googleCharts.load().catch(() => {});
  }

  @action loading(transition) {
    this.progress.handle(transition);
    return true;
  }
}
