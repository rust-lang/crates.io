import { module, test } from 'qunit';
import { setupTest } from 'ember-qunit';
import window, { setupWindowMock } from 'ember-window-mock';

const URL = 'https://turbo.fish/';

module('Service | Redirector', function(hooks) {
  setupTest(hooks);
  setupWindowMock(hooks);

  test('redirectTo() sets `window.location`', function(assert) {
    assert.notEqual(window.location.href, URL);

    let redirector = this.owner.lookup('service:redirector');
    redirector.redirectTo(URL);
    assert.equal(window.location.href, URL);
  });
});
