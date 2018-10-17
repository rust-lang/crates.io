import Route from '@ember/routing/route';

export default Route.extend({
    afterModel(crate) {
        if (crate === undefined) this.transitionTo('crates');
        else this.transitionTo('crate', crate);
    },
});
