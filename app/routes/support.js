import Route from '@ember/routing/route';

export default class CrateRoute extends Route {
  resetController(controller, isExiting) {
    super.resetController(...arguments);
    // reset queryParams when exiting
    if (isExiting) {
      for (let param of controller.queryParams) {
        controller.set(param, null);
      }
    }
  }
}
