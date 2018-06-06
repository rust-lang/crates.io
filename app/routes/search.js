import Route from '@ember/routing/route';

export default Route.extend({
    queryParams: {
        q: { refreshModel: true },
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        // we need a model() implementation that changes, otherwise the setupController() hook
        // is not called and we won't reload the results if a new query string is used
        return params;
    },

    setupController(controller, params) {
        controller.dataTask.perform(params);
    },
});
