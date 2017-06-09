import Ember from 'ember';
import Resolver from './resolver';
import loadInitializers from 'ember-load-initializers';
import config from './config/environment';
import RouteAliasResolverMixin from 'cargo/mixins/route-alias-resolver';

let App;

let CargoResolver = Resolver.extend(RouteAliasResolverMixin);

App = Ember.Application.extend({
    modulePrefix: config.modulePrefix,
    podModulePrefix: config.podModulePrefix,
    Resolver: CargoResolver
});

loadInitializers(App, config.modulePrefix);

Ember.$.ajaxSetup({
    cache: false
});

export default App;
