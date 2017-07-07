export function initialize(application) {
    application.inject('controller', 'session', 'service:session');
    application.inject('route', 'session', 'service:session');
}

export default {
    name: 'app.session',
    initialize
};
