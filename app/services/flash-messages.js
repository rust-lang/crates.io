import Service from '@ember/service';

export default class FlashMessagesService extends Service {
  message = null;
  _nextMessage = null;

  show(message, options = { type: 'warning' }) {
    this.set('message', message);
    this.set('options', options);
  }

  queue(message) {
    this.set('_nextMessage', message);
  }

  step() {
    this.set('message', this._nextMessage);
    this.set('_nextMessage', null);
  }
}
