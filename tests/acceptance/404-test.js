import { currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Acceptance | 404', function (hooks) {
  setupApplicationTest(hooks);

  test('/unknown-route shows a 404 page', async function (assert) {
    await visit('/unknown-route');
    assert.equal(currentURL(), '/unknown-route');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('Page not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();

    await percySnapshot(assert);
  });
});
