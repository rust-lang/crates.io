import Route from '@ember/routing/route';

export default Route.extend({
    setupController(controller) {
        this._super(...arguments);
        const crate = this.modelFor('crate');
        controller.set('crate', crate);
    },
});
