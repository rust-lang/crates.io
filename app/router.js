import Ember from 'ember';
import config from './config/environment';
import googlePageview from 'cargo/mixins/google-pageview';

var Router = Ember.Router.extend(googlePageview, {
  location: config.locationType
});

Router.map(function() {
  this.route('logout', { resetNamespace: true });
  this.route('login', { resetNamespace: true });
  this.route('github_login', { resetNamespace: true });
  this.route('github_authorize', { path: '/authorize/github', resetNamespace: true });
  this.route('crates', { resetNamespace: true });
  this.route('crate', { path: '/crates/*crate_id', resetNamespace: true }, function() {
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
  this.route('keywords', { resetNamespace: true });
  this.route('keyword', { path: '/keywords/*keyword_id', resetNamespace: true }, function() {
  });
  this.route('catchAll', { path: '*path' });
});

export default Router;
