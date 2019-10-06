import Service from '@ember/service';

export default Service.extend({
  message: null,
  _nextMessage: null,

  show(message) {
    this.set('message', message);
  },

  queue(message) {
    this.set('_nextMessage', message);
  },

  step() {
    this.set('message', this._nextMessage);
    this.set('_nextMessage', null);
  },
});
