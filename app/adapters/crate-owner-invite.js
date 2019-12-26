import RESTAdapter from '@ember-data/adapter/rest';

export default RESTAdapter.extend({
  namespace: 'api/v1/me',
  pathForType() {
    return 'crate_owner_invitations';
  },
});
