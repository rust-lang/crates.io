import Ember from 'ember';
import config from './config/environment';
import googlePageview from 'cargo/mixins/google-pageview';

var Router = Ember.Router.extend(googlePageview, {
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
    this.route('versions');
    this.route('reverse_dependencies');

    // Well-known routes
    this.route('docs');
  });
  this.route('me', function() {
    this.route('crates');
    this.route('following');
  });
  this.route('install');
  this.route('search');
  this.route('dashboard');
  this.resource('keywords');
  this.resource('keyword', { path: '/keywords/*keyword_id' }, function() {
  });
  this.route('catchAll', { path: '*path' });
});

export default Router;
