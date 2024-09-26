import { currentURL, findAll } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../axe-config';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | support', function (hooks) {
  setupApplicationTest(hooks);

  test('shows an inquire list', async function (assert) {
    await visit('/support');
    assert.strictEqual(currentURL(), '/support');

    assert.dom('[data-test-id="support-main-content"] section').exists({ count: 1 });
    assert.dom('[data-test-id="inquire-list-section"]').exists();
    assert.dom('[data-test-id="inquire-list"]').exists();
    const listitem = findAll('[data-test-id="inquire-list"] li');
    assert.deepEqual(
      listitem.map(item => item.textContent.trim()),
      ['Report a crate that violates policies'],
    );

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('shows an inquire list if given inquire is not supported', async function (assert) {
    await visit('/support?inquire=not-supported-inquire');
    assert.strictEqual(currentURL(), '/support?inquire=not-supported-inquire');

    assert.dom('[data-test-id="support-main-content"] section').exists({ count: 1 });
    assert.dom('[data-test-id="inquire-list-section"]').exists();
    assert.dom('[data-test-id="inquire-list"]').exists();
    const listitem = findAll('[data-test-id="inquire-list"] li');
    assert.deepEqual(
      listitem.map(item => item.textContent.trim()),
      ['Report a crate that violates policies'],
    );
  });
});
