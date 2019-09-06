import RSVP from 'rsvp';

export function initialize() {
  // async/await is using window.Promise by default and we want async/await to
  // use RSVP instead which is properly integrated with Ember's runloop
  window.Promise = RSVP.Promise;
}

export default { initialize };
