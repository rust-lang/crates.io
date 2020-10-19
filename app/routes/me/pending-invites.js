import AuthenticatedRoute from '../-authenticated-route';

export default class PendingInvitesRoute extends AuthenticatedRoute {
  model() {
    return this.store.findAll('crate-owner-invite');
  }
}
