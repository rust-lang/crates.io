import Route from '@ember/routing/route';

export default class IndexRoute extends Route {
  setupController(controller) {
    if (!controller.hasData) {
      controller.fetchData();
    }
  }
}
