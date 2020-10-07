import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  googleCharts: service(),
  progress: service(),
  session: service(),

  beforeModel() {
    // trigger the task, but don't wait for the result here
    this.session.loadUserTask.perform();

    // start loading the Google Charts JS library already
    // and ignore any errors since we will catch them again
    // anyway when we call `load()` from the `DownloadGraph`
    // component
    this.googleCharts.load().catch(() => {});
  },

  actions: {
    loading(transition) {
      this.progress.handle(transition);
      return true;
    },
  },
});
