import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

module('Unit | Helper | pretty-bytes', function (hooks) {
  setupRenderingTest(hooks);

  test("it displays as expected", async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await render(hbs`{{pretty-bytes 42}}`);
    assert.dom().hasText('42.0 B');

    await render(hbs`{{pretty-bytes 42000}}`);
    assert.dom().hasText('41.0 KiB');

    await render(hbs`{{pretty-bytes 42420}}`);
    assert.dom().hasText('41.4 KiB');

    await render(hbs`{{pretty-bytes 42424242}}`);
    assert.dom().hasText('40.5 MiB');
  })
});
