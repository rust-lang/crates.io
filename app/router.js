import Ember from 'ember';

var Router = Ember.Router.extend({
  location: CargoENV.locationType
});

Router.map(function() {
  this.resource('logout');
  this.resource('login');
  this.resource('github_login');
  this.resource('github_authorize', { path: '/authorize/github' });
  this.resource('packages');
  this.resource('package', { path: '/packages/:package_id' }, function() {
    this.route('edit');
  });
  this.route('me');
});

export default Router;
