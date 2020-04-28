import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  fastboot: service(),

  headTags() {
    return [
      {
        type: 'meta',
        attrs: {
          name: 'description',
          content: 'cargo is the package manager and crate host for rust',
        },
      },
    ];
  },

  setupController(controller) {
    this.controllerFor('application').set('searchQuery', null);

    let promise = controller.dataTask.perform();
    if (this.fastboot.isFastBoot) {
      this.fastboot.deferRendering(promise);
    }
  },
});
