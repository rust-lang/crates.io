import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
    stats(id) {
        return this.ajax(this.urlForStatsAction(id), 'GET');
    },

    urlForStatsAction(id) {
        return `${this.buildURL('user', id)}/stats`;
    },
});
