import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
    getDownloadUrl(dlPath) {
        return this.ajax(dlPath, 'GET').then(response => response.url);
    },
});
