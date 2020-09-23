import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
  namespace: 'api/v1/me',
  pathForType() {
    return 'crate_owner_invitations';
  },
});
