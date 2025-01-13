import { click, currentURL, fillIn } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | crate deletion', function (hooks) {
  setupApplicationTest(hooks);

  test('happy path', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate });
    this.server.create('crate-ownership', { crate, user });

    await visit('/crates/foo');
    assert.strictEqual(currentURL(), '/crates/foo');
    assert.dom('[data-test-settings-tab] a').exists();

    await click('[data-test-settings-tab] a');
    assert.strictEqual(currentURL(), '/crates/foo/settings');
    assert.dom('[data-test-delete-button]').exists();

    await click('[data-test-delete-button]');
    assert.strictEqual(currentURL(), '/crates/foo/delete');
    assert.dom('[data-test-title]').hasText('Delete the foo crate?');
    assert.dom('[data-test-delete-button]').isDisabled();

    await fillIn('[data-test-reason]', "I don't need this crate anymore");
    await click('[data-test-confirmation-checkbox]');
    assert.dom('[data-test-delete-button]').isEnabled();

    await click('[data-test-delete-button]');
    assert.strictEqual(currentURL(), '/');

    let message = 'Crate foo has been successfully deleted.';
    assert.dom('[data-test-notification-message="success"]').hasText(message);

    crate = this.server.schema.crates.findBy({ name: 'foo' });
    assert.strictEqual(crate, null);
  });
});
