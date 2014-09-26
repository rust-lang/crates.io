import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  model: function() {
    var self = this;

    var addPackages = function(pkgs) {
        for (var i = 0; i < pkgs.length; i++) {
            pkgs[i] = self.store.push('package', pkgs[i]);
        }
    };
    return ajax('/summary').then(function(data) {
        addPackages(data.new_packages);
        addPackages(data.most_downloaded);
        addPackages(data.just_updated);
        console.log(data.new_packages);
        return data;
    });
  }
});

