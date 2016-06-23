export default {
    name: 'app.session',

    initialize(application) {
        application.inject('controller', 'session', 'service:session');
        application.inject('route', 'session', 'service:session');
    }
};
