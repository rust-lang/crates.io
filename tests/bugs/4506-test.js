import { click } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Bug #4506', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let { server } = context;

    server.create('keyword', { keyword: 'no-std' });

    let foo = server.create('crate', { name: 'foo', keywordIds: ['no-std'] });
    server.create('version', { crate: foo });

    let bar = server.create('crate', { name: 'bar', keywordIds: ['no-std'] });
    server.create('version', { crate: bar });
  }

  test('is fixed', async function (assert) {
    prepare(this);

    await visit('/crates/foo');
    assert.dom('[data-test-keyword]').exists({ count: 1 });

    await click('[data-test-keyword="no-std"]');
    assert.dom('[data-test-total-rows]').hasText('2');
    assert.dom('[data-test-crate-row]').exists({ count: 2 });
  });

  test('is fixed for /keywords too', async function (assert) {
    prepare(this);

    await visit('/keywords');
    assert.dom('[data-test-keyword]').exists({ count: 1 });
    assert.dom('[data-test-keyword="no-std"] [data-test-count]').hasText('2 crates');

    await click('[data-test-keyword="no-std"] a');
    assert.dom('[data-test-total-rows]').hasText('2');
    assert.dom('[data-test-crate-row]').exists({ count: 2 });
  });
});
