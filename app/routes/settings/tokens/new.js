import Route from '@ember/routing/route';

export default class TokenListRoute extends Route {
  resetController(controller) {
    controller.saveTokenTask.cancelAll();
  }
}
