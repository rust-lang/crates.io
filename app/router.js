import Ember from 'ember';
import config from './config/environment';
import googlePageview from './mixins/google-pageview';

const Router = Ember.Router.extend(googlePageview, {
    location: config.locationType
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
        this.alias('index', '/', 'version');

        this.route('reverse_dependencies');

        // Well-known routes
        this.route('docs');
    });
    this.route('me', function() {
        this.route('crates');
        this.route('following');
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
    this.route('catchAll', { path: '*path' });
});

export default Router;
