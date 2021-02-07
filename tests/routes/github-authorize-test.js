import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Route | github-authorized', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);

  test('forwards code and state to window.opener.postMessage()', async function (assert) {
    let message = null;
    window.opener = {
      postMessage(_message) {
        assert.step('window.opener.postMessage()');
        message = _message;
      },
    };

    await visit('/authorize/github?code=901dd10e07c7e9fa1cd5&state=fYcUY3FMdUUz00FC7vLT7A');

    assert.deepEqual(message, {
      code: '901dd10e07c7e9fa1cd5',
      state: 'fYcUY3FMdUUz00FC7vLT7A',
    });

    assert.verifySteps(['window.opener.postMessage()']);
  });
});
