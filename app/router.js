import EmberRouter from '@ember/routing/router';

import config from './config/environment';

export default class Router extends EmberRouter {
  location = config.locationType;
  rootURL = config.rootURL;
}

Router.map(function () {
  this.route('crates');
  this.route('crate', { path: '/crates/:crate_id' }, function () {
    this.route('versions');
    this.route('dependencies');
    this.route('version', { path: '/:version_num' });
    this.route('version-dependencies', { path: '/:version_num/dependencies' });
    this.route('rebuild-docs', { path: '/:version_num/rebuild-docs' });
    this.route('range', { path: '/range/:range' });

    this.route('reverse-dependencies', { path: 'reverse_dependencies' });

    this.route('owners');
    this.route('settings', function () {
      this.route('new-trusted-publisher');
    });
    this.route('delete');

    // Well-known routes
    this.route('docs');
    this.route('repo');
  });
  this.route('me', function () {
    this.route('crates');
    this.route('following');
    this.route('pending-invites');
  });
  this.route('settings', function () {
    this.route('email-notifications');
    this.route('profile');
    this.route('tokens', function () {
      this.route('new');
    });
  });
  this.route('user', { path: '/users/:user_id' });
  this.route('install');
  this.route('search');
  this.route('dashboard');
  this.route('keywords');
  this.route('keyword', { path: '/keywords/:keyword_id' }, function () {
    this.route('index', { path: '/' });
  });
  this.route('categories');
  this.route('category', { path: '/categories/:category_id' }, function () {
    this.route('index', { path: '/' });
  });
  this.route('category-slugs', { path: 'category_slugs' });
  this.route('team', { path: '/teams/:team_id' });
  this.route('policies', function () {
    this.route('security');
  });
  // The canonical security page is `/policies/security`; this is a redirect to support guessing
  // of the URL to be `/security`.
  this.route('security');
  this.route('data-access');
  this.route('docs', function () {
    this.route('trusted-publishing');
  });
  this.route('confirm', { path: '/confirm/:email_token' });
  this.route('accept-invite', { path: '/accept-invite/:token' });
  this.route('support');

  this.route('catch-all', { path: '*path' });
});
