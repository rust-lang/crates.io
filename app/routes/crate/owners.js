import Route from '@ember/routing/route';

export default Route.extend({
    setupController(controller) {
        this._super(...arguments);
        let crate = this.modelFor('crate');
        controller.set('crate', crate);
    },
});
