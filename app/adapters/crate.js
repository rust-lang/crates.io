import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
    follow(id) {
        return this.ajax(this.urlForFollowAction(id), 'PUT');
    },

    unfollow(id) {
        return this.ajax(this.urlForFollowAction(id), 'DELETE');
    },

    urlForFollowAction(id) {
        return `${this.buildURL('crate', id)}/follow`;
    },
});
