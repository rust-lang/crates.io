import Ember from 'ember';

export default Ember.Controller.extend({
    actions: {
        search: function(query) {
            return this.transitionToRoute('search', {queryParams: {q: query}});
        },
    },
});
