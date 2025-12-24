import { click, currentURL, findAll } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | rebuild docs page', function (hooks) {
  setupApplicationTest(hooks);

  test('navigates to rebuild docs confirmation page', async function (assert) {
    let user = await this.db.user.create({});
    await this.authenticateAs(user);

    let crate = await this.db.crate.create({ name: 'nanomsg' });
    await this.db.crateOwnership.create({ crate, user });

    await this.db.version.create({ crate, num: '0.1.0', created_at: '2017-01-01' });
    await this.db.version.create({ crate, num: '0.2.0', created_at: '2018-01-01' });
    await this.db.version.create({ crate, num: '0.3.0', created_at: '2019-01-01', rust_version: '1.69' });
    await this.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await visit('/crates/nanomsg/versions');
    assert.strictEqual(currentURL(), '/crates/nanomsg/versions');

    let versions = findAll('[data-test-version]').map(it => it.dataset.testVersion);
    assert.deepEqual(versions, ['0.2.1', '0.3.0', '0.2.0', '0.1.0']);

    await click('[data-test-version="0.2.1"] [data-test-actions-toggle]');
    await click('[data-test-version="0.2.1"] [data-test-id="btn-rebuild-docs"]');

    assert.strictEqual(currentURL(), '/crates/nanomsg/0.2.1/rebuild-docs');
    assert.dom('[data-test-title]').hasText('Rebuild Documentation');
  });

  test('rebuild docs confirmation page shows crate info and allows confirmation', async function (assert) {
    let user = await this.db.user.create({});
    await this.authenticateAs(user);

    let crate = await this.db.crate.create({ name: 'nanomsg' });
    await this.db.crateOwnership.create({ crate, user });

    await this.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await visit('/crates/nanomsg/0.2.1/rebuild-docs');
    assert.strictEqual(currentURL(), '/crates/nanomsg/0.2.1/rebuild-docs');

    assert.dom('[data-test-title]').hasText('Rebuild Documentation');
    assert.dom('[data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-version-num]').hasText('0.2.1');

    await click('[data-test-confirm-rebuild-button]');

    let message = 'Docs rebuild task was enqueued successfully!';
    assert.dom('[data-test-notification-message="success"]').hasText(message);
    assert.strictEqual(currentURL(), '/crates/nanomsg/versions');
  });

  test('rebuilds docs confirmation page redirects non-owners to error page', async function (assert) {
    let user = await this.db.user.create({});
    await this.authenticateAs(user);

    let crate = await this.db.crate.create({ name: 'nanomsg' });
    await this.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await visit('/crates/nanomsg/0.2.1/rebuild-docs');
    assert.dom('[data-test-title]').hasText('This page is only accessible by crate owners');
    assert.dom('[data-test-go-back]').exists();
  });

  test('rebuild docs confirmation page shows authentication error for unauthenticated users', async function (assert) {
    let crate = await this.db.crate.create({ name: 'nanomsg' });
    await this.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await visit('/crates/nanomsg/0.2.1/rebuild-docs');

    // Unauthenticated users should see authentication error
    assert.strictEqual(currentURL(), '/crates/nanomsg/0.2.1/rebuild-docs');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });
});
