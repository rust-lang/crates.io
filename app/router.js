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
  this.resource('crate', { path: '/crates/*crate_id' }, function() {
    this.route('download');
  });
  this.route('me');
  this.route('guide');
  this.route('install');
  this.route('search');
});

export default Router;
