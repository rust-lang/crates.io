import window from 'ember-window-mock';

export default function (server) {
  let user = server.create('user');
  server.create('mirage-session', { user });
  window.localStorage.setItem('isLoggedIn', '1');

  server.loadFixtures();
}
