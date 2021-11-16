import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

/**
 * This is a weird route... but let me explain.
 *
 * This is the default route that gets used if no other matching route is found.
 *
 * This route is *also* used as a generic error page via:
 *
 * ```js
 * this.router.replaceWith('catch-all', { transition, title: 'Something failed' });
 * ```
 *
 * Ideally we would use the `error` substate/routes of Ember.js, but those don't
 * update the URL when an error happens. This causes the native back button of the
 * browser to behave in strange way, so we avoid using the broken built-in error
 * routes.
 */
export default class CatchAllRoute extends Route {
  @service router;

  /**
   * If `transitionTo('catch-all', 'foo')` is used, this hook will not get called.
   * If the second argument is an object, then the second object will be the `model`
   * of this route, and the `serialize()` hook gets called to figure out what the
   * URL of this route should be. The URL is automatically assembled from the passed-in
   * transition object.
   */
  serialize({ transition }) {
    return { path: this.pathForRouteInfo(transition.to) };
  }

  /**
   * This internal method takes a `RouteInfo` object from Ember.js (e.g. `transition.to`)
   * and returns the corresponding `:path` route parameter for this `catch-all` route.
   * @return {string}
   */
  pathForRouteInfo(routeInfo) {
    let routeName = routeInfo.name;
    let params = paramsForRouteInfo(routeInfo);
    let queryParams = routeInfo.queryParams;
    return this.router.urlFor(routeName, ...params, { queryParams }).slice(1);
  }
}

/**
 * Returns all route parameters for the passed-in `RouteInfo` object.
 *
 * These can be used in `router.urlFor(...)` calls.
 */
function paramsForRouteInfo(routeInfo) {
  let routeInfos = [...allRouteInfos(routeInfo)].reverse();

  let params = [];
  for (let routeInfo of routeInfos) {
    for (let paramName of routeInfo.paramNames) {
      params.push(routeInfo.params[paramName]);
    }
  }
  return params;
}

/**
 * Iterates upwards through the `RouteInfo` "family tree" until the top-most
 * `RouteInfo` is reached.
 */
function* allRouteInfos(routeInfo) {
  yield routeInfo;
  while ((routeInfo = routeInfo.parent)) {
    yield routeInfo;
  }
}
