import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
    stats(id) {
        return this.ajax(this.urlForStatsAction(id), 'GET');
    },

    urlForStatsAction(id) {
        return `${this.buildURL('user', id)}/stats`;
    },

    queryRecord(store, type, query) {
        let url = this.urlForFindRecord(query.user_id, 'user');
        return this.ajax(url, 'GET');
    },

    favorite(id) {
        return this.ajax(this.urlForFavoriteAction(id), 'PUT');
    },

    unfavorite(id) {
        return this.ajax(this.urlForFavoriteAction(id), 'DELETE');
    },

    urlForFavoriteAction(id) {
        return `${this.buildURL('user', id)}/favorite`;
    },

    favoriteUsers(id) {
        return this.ajax(`${this.buildURL('user', id)}/favorite_users`, 'GET');
    },
});
