import ApplicationAdapter from './application';

export default class CrateOwnerInviteAdapter extends ApplicationAdapter {
  namespace = 'api/v1/me';

  pathForType() {
    return 'crate_owner_invitations';
  }

  urlForQuery() {
    return '/api/private/crate_owner_invitations';
  }
}
