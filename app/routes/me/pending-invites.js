import AuthenticatedRoute from '../-authenticated-route';

export default AuthenticatedRoute.extend({
  model() {
    return this.store.findAll('crate-owner-invite');
  },
});
