import { click, currentURL, settled, visit, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { getPageTitle } from 'ember-page-title/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { summary } from '../../mirage/route-handlers/summary';
import axeConfig from '../axe-config';

module('Acceptance | front page', function (hooks) {
  setupApplicationTest(hooks);

  test('visiting /', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    this.server.loadFixtures();

    await visit('/');

    assert.equal(currentURL(), '/');
    assert.equal(getPageTitle(), 'crates.io: Rust Package Registry');

    assert.dom('[data-test-install-cargo-link]').exists();
    assert.dom('[data-test-all-crates-link]').exists();
    assert.dom('[data-test-login-button]').exists();

    assert.dom('[data-test-total-downloads] [data-test-value]').hasText('143,345');
    assert.dom('[data-test-total-crates] [data-test-value]').hasText('23');

    assert.dom('[data-test-new-crates] [data-test-crate-link="0"]').hasText('Inflector v1.0.0');
    assert.dom('[data-test-new-crates] [data-test-crate-link="0"]').hasAttribute('href', '/crates/Inflector');

    assert.dom('[data-test-most-downloaded] [data-test-crate-link="0"]').hasText('serde');
    assert.dom('[data-test-most-downloaded] [data-test-crate-link="0"]').hasAttribute('href', '/crates/serde');

    assert.dom('[data-test-just-updated] [data-test-crate-link="0"]').hasText('nanomsg v0.6.1');
    assert.dom('[data-test-just-updated] [data-test-crate-link="0"]').hasAttribute('href', '/crates/nanomsg');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('error handling', async function (assert) {
    this.server.get('/api/v1/summary', {}, 500);

    await visit('/');
    assert.dom('[data-test-lists]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isEnabled();

    let deferred = defer();
    this.server.get('/api/v1/summary', async function (schema, request) {
      await deferred.promise;
      return summary.call(this, schema, request);
    });

    click('[data-test-try-again-button]');
    await waitFor('[data-test-try-again-button] [data-test-spinner]');
    assert.dom('[data-test-lists]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isDisabled();

    deferred.resolve();
    await settled();
    assert.dom('[data-test-lists]').exists();
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-try-again-button]').doesNotExist();
  });
});
