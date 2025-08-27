import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import prettyBytes from 'crates-io/helpers/pretty-bytes';
import { setupRenderingTest } from 'crates-io/tests/helpers';

module('Unit | Helper | pretty-bytes', function (hooks) {
  setupRenderingTest(hooks);

  test('it displays as expected', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await render(<template>{{prettyBytes 42}}</template>);
    assert.dom().hasText('42 B');

    await render(<template>{{prettyBytes 1024}}</template>);
    assert.dom().hasText('1 KiB');

    // 4200 / 1024 = 4.101...
    await render(<template>{{prettyBytes 4200}}</template>);
    assert.dom().hasText('4.1 KiB');

    // 4200 / 1024 = 4.142...
    await render(<template>{{prettyBytes 4242}}</template>);
    assert.dom().hasText('4.14 KiB');

    // 42000 / 1024 = 41.0156...
    await render(<template>{{prettyBytes 42000}}</template>);
    assert.dom().hasText('41 KiB');

    // 42623 / 1024 = 41.625
    await render(<template>{{prettyBytes 42624}}</template>);
    assert.dom().hasText('41.6 KiB');

    // 424242 / 1024 = 414.2988...
    await render(<template>{{prettyBytes 424242}}</template>);
    assert.dom().hasText('414 KiB');
  });
});
