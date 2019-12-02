import Route from '@ember/routing/route';

export default Route.extend({
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
    controller.dataTask.perform();
  },
});
