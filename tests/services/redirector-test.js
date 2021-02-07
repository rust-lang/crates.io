import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupTest } from 'cargo/tests/helpers';

const URL = 'https://turbo.fish/';

module('Service | Redirector', function (hooks) {
  setupTest(hooks);
  setupWindowMock(hooks);

  test('redirectTo() sets `window.location`', function (assert) {
    assert.notEqual(window.location.href, URL);

    let redirector = this.owner.lookup('service:redirector');
    redirector.redirectTo(URL);
    assert.equal(window.location.href, URL);
  });
});
