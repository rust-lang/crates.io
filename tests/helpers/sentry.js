import Service from '@ember/service';

class MockSentryService extends Service {
  events = [];
  scope = new MockScope();

  captureException(error) {
    let { scope, user } = this;
    let { tags } = scope;
    let event = { error, tags, user };
    this.events.push(event);
  }

  configureScope(callback) {
    callback(this.scope);
  }

  setUser(user) {
    this.user = user;
  }
}

class MockScope {
  tags = {};

  setTag(key, value) {
    this.tags[key] = value;
  }
}

export function setupSentryMock(hooks) {
  hooks.beforeEach(function () {
    this.owner.register('service:sentry', MockSentryService);
  });
}
