import EmberRouter from '@ember/routing/router';
import config from './config/environment';
import googlePageview from './mixins/google-pageview';
import RouterScroll from 'ember-router-scroll';

const Router = EmberRouter.extend(googlePageview, RouterScroll, {
    location: config.locationType,
    rootURL: config.rootURL
});

Router.map(function() {
    this.route('logout');
    this.route('login');
    this.route('github_login');
    this.route('github_authorize', { path: '/authorize/github' });
    this.route('crates');
    this.route('crate', { path: '/crates/:crate_id' }, function() {
        this.route('download');
        this.route('versions');
        this.route('version', { path: '/:version_num' });

        this.route('reverse_dependencies');

        // Well-known routes
        this.route('docs');
        this.route('repo');
    });
    this.route('me', function() {
        this.route('crates');
        this.route('following');
        this.route('pending-invites');
    });
    this.route('user', { path: '/users/:user_id' });
    this.route('install');
    this.route('search');
    this.route('dashboard');
    this.route('keywords');
    this.route('keyword', { path: '/keywords/:keyword_id' }, function() {
        this.route('index', { path: '/' });
    });
    this.route('categories');
    this.route('category', { path: '/categories/:category_id' }, function() {
        this.route('index', { path: '/' });
    });
    this.route('category_slugs');
    this.route('catchAll', { path: '*path' });
    this.route('team', { path: '/teams/:team_id' });
    this.route('policies');
    this.route('confirm', { path: '/confirm/:email_token' });
});

export default Router;
