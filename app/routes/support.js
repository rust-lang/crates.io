import AuthenticatedRoute from './-authenticated-route';

export default class CrateRoute extends AuthenticatedRoute {
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
