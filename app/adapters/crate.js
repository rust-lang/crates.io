import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
  follow(id) {
    return this.ajax(this.urlForFollowAction(id), 'PUT');
  },

  async inviteOwner(id, username) {
    let result = await this.ajax(this.urlForOwnerAction(id), 'PUT', {
      data: {
        owners: [username],
      },
    });

    if (result.ok) {
      return result;
    } else {
      throw result;
    }
  },

  removeOwner(id, username) {
    return this.ajax(this.urlForOwnerAction(id), 'DELETE', {
      data: {
        owners: [username],
      },
    });
  },

  unfollow(id) {
    return this.ajax(this.urlForFollowAction(id), 'DELETE');
  },

  urlForFollowAction(id) {
    return `${this.buildURL('crate', id)}/follow`;
  },

  urlForOwnerAction(id) {
    return `${this.buildURL('crate', id)}/owners`;
  },
});
