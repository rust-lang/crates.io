export default {
  name: 'app.session',

  initialize: function(container, application) {
    application.inject('controller', 'session', 'service:session');
    application.inject('route', 'session', 'service:session');
  }
};
