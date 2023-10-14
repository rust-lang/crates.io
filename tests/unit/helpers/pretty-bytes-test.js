import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'crates-io/tests/helpers';

module('Unit | Helper | pretty-bytes', function (hooks) {
  setupRenderingTest(hooks);

  test('it displays as expected', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await render(hbs`{{pretty-bytes 42}}`);
    assert.dom().hasText('42 B');

    await render(hbs`{{pretty-bytes 1024}}`);
    assert.dom().hasText('1 KiB');

    // 4200 / 1024 = 4.101...
    await render(hbs`{{pretty-bytes 4200}}`);
    assert.dom().hasText('4.1 KiB');

    // 4200 / 1024 = 4.142...
    await render(hbs`{{pretty-bytes 4242}}`);
    assert.dom().hasText('4.14 KiB');

    // 42000 / 1024 = 41.0156...
    await render(hbs`{{pretty-bytes 42000}}`);
    assert.dom().hasText('41 KiB');

    // 42623 / 1024 = 41.625
    await render(hbs`{{pretty-bytes 42624}}`);
    assert.dom().hasText('41.6 KiB');

    // 424242 / 1024 = 414.2988...
    await render(hbs`{{pretty-bytes 424242}}`);
    assert.dom().hasText('414 KiB');
  });
});
