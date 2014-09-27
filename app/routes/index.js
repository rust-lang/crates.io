import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  model: function() {
    var self = this;

    var addCrates = function(crates) {
        for (var i = 0; i < crates.length; i++) {
            crates[i] = self.store.push('crate', crates[i]);
        }
    };
    return ajax('/summary').then(function(data) {
        addCrates(data.new_crates);
        addCrates(data.most_downloaded);
        addCrates(data.just_updated);
        return data;
    });
  }
});

