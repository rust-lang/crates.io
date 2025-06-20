import Route from '@ember/routing/route';

export default class NewTrustedPublisherRoute extends Route {
  async model() {
    let crate = this.modelFor('crate');
    return { crate };
  }
}
