import Ember from 'ember';
import config from './config/environment';

var Router = Ember.Router.extend({
  location: config.locationType
});

Router.map(function() {
  this.resource('logout');
  this.resource('login');
  this.resource('github_login');
  this.resource('github_authorize', { path: '/authorize/github' });
  this.resource('crates');
  this.resource('crate', { path: '/crates/:crate_id' });
  this.route('me');
  this.route('download');
  this.route('guide');
});

export default Router;
