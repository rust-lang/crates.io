import Ember from 'ember';

export default Ember.ObjectController.extend({
    sortedVersions: function() {
        return this.get('model.versions').sortBy('num').reverse();
    }.property('model.versions.@each'),
});
