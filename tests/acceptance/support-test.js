import { click, currentURL, fillIn, findAll } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

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

  module('reporting a crate from support page', function () {
    setupWindowMock(hooks);

    async function prepare(context, assert) {
      let server = context.server;
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });

      window.open = (url, target, features) => {
        window.openKwargs = { url, target, features };
        return { document: { write() {}, close() {} }, close() {} };
      };

      await visit('/support');
      await click('[data-test-id="link-crate-violation"]');
      assert.strictEqual(currentURL(), '/support?inquire=crate-violation');
    }

    test('show a report form', async function (assert) {
      await prepare(this, assert);

      assert.dom('[data-test-id="support-main-content"] section').exists({ count: 1 });
      assert.dom('[data-test-id="crate-violation-section"]').exists();
      assert.dom('[data-test-id="fieldset-crate"]').exists();
      assert.dom('[data-test-id="fieldset-reasons"]').exists();
      assert.dom('[data-test-id="fieldset-detail"]').exists();
      assert.dom('[data-test-id="report-button"]').hasText('Report');

      await percySnapshot(assert);
      await a11yAudit(axeConfig);
    });

    test('empty form should shows errors', async function (assert) {
      await prepare(this, assert);
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').exists();
      assert.dom('[data-test-id="reasons-invalid"]').exists();
      assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

      assert.strictEqual(window.openKwargs, undefined);
    });

    test('empty crate should shows errors', async function (assert) {
      await prepare(this, assert);
      assert.dom('[data-test-id="crate-input"]').hasValue('');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').exists();
      assert.dom('[data-test-id="reasons-invalid"]').exists();
      assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

      assert.strictEqual(window.openKwargs, undefined);
    });

    test('other reason selected without given detail shows an error', async function (assert) {
      await prepare(this, assert);
      await fillIn('[data-test-id="crate-input"]', 'nanomsg');
      assert.dom('[data-test-id="crate-input"]').hasValue('nanomsg');

      await click('[data-test-id="spam-checkbox"]');
      assert.dom('[data-test-id="spam-checkbox"]').isChecked();
      await click('[data-test-id="other-checkbox"]');
      assert.dom('[data-test-id="other-checkbox"]').isChecked();
      assert.dom('[data-test-id="detail-input"]').hasValue('');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').doesNotExist();
      assert.dom('[data-test-id="reasons-invalid"]').doesNotExist();
      assert.dom('[data-test-id="detail-invalid"]').exists();

      assert.strictEqual(window.openKwargs, undefined);
    });

    test('valid form without detail', async function (assert) {
      await prepare(this, assert);
      await fillIn('[data-test-id="crate-input"]', 'nanomsg');
      assert.dom('[data-test-id="crate-input"]').hasValue('nanomsg');

      await click('[data-test-id="spam-checkbox"]');
      assert.dom('[data-test-id="spam-checkbox"]').isChecked();
      assert.dom('[data-test-id="detail-input"]').hasValue('');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').doesNotExist();
      assert.dom('[data-test-id="reasons-invalid"]').doesNotExist();
      assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

      let body = `I'm reporting the https://crates.io/crates/nanomsg crate because:

- [x] it contains spam
- [ ] it is name-squatting (reserving a crate name without content)
- [ ] it is abusive or otherwise harmful
- [ ] it contains a vulnerability (please try to contact the crate author first)
- [ ] it is violating the usage policy in some other way (please specify below)

Additional details:


`;
      let subject = `The "nanomsg" crate`;
      let address = 'help@crates.io';
      let mailto = `mailto:${address}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
      assert.true(!!window.openKwargs);
      assert.strictEqual(window.openKwargs.url, mailto);
      assert.strictEqual(window.openKwargs.target, '_self');
    });

    test('valid form with required detail', async function (assert) {
      await prepare(this, assert);
      await fillIn('[data-test-id="crate-input"]', 'nanomsg');
      assert.dom('[data-test-id="crate-input"]').hasValue('nanomsg');

      await click('[data-test-id="spam-checkbox"]');
      assert.dom('[data-test-id="spam-checkbox"]').isChecked();
      await click('[data-test-id="other-checkbox"]');
      assert.dom('[data-test-id="other-checkbox"]').isChecked();
      await fillIn('[data-test-id="detail-input"]', 'test detail');
      assert.dom('[data-test-id="detail-input"]').hasValue('test detail');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').doesNotExist();
      assert.dom('[data-test-id="reasons-invalid"]').doesNotExist();
      assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

      let body = `I'm reporting the https://crates.io/crates/nanomsg crate because:

- [x] it contains spam
- [ ] it is name-squatting (reserving a crate name without content)
- [ ] it is abusive or otherwise harmful
- [ ] it contains a vulnerability (please try to contact the crate author first)
- [x] it is violating the usage policy in some other way (please specify below)

Additional details:

test detail
`;
      let subject = `The "nanomsg" crate`;
      let address = 'help@crates.io';
      let mailto = `mailto:${address}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
      assert.true(!!window.openKwargs);
      assert.strictEqual(window.openKwargs.url, mailto);
      assert.strictEqual(window.openKwargs.target, '_self');
    });
  });

  module('reporting a crate from crate page', function () {
    setupWindowMock(hooks);

    async function prepare(context, assert) {
      let server = context.server;
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });

      window.open = (url, target, features) => {
        window.openKwargs = { url, target, features };
        return { document: { write() {}, close() {} }, close() {} };
      };

      await visit('/crates/nanomsg');
      await click('[data-test-id="link-crate-report"]');
      assert.strictEqual(currentURL(), '/support?crate=nanomsg&inquire=crate-violation');
      assert.dom('[data-test-id="crate-input"]').hasValue('nanomsg');
    }

    test('empty crate should shows errors', async function (assert) {
      await prepare(this, assert);
      await fillIn('[data-test-id="crate-input"]', '');
      assert.dom('[data-test-id="crate-input"]').hasValue('');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').exists();
      assert.dom('[data-test-id="reasons-invalid"]').exists();
      assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

      assert.strictEqual(window.openKwargs, undefined);
    });

    test('other reason selected without given detail shows an error', async function (assert) {
      await prepare(this, assert);

      await click('[data-test-id="spam-checkbox"]');
      assert.dom('[data-test-id="spam-checkbox"]').isChecked();
      await click('[data-test-id="other-checkbox"]');
      assert.dom('[data-test-id="other-checkbox"]').isChecked();
      assert.dom('[data-test-id="detail-input"]').hasValue('');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').doesNotExist();
      assert.dom('[data-test-id="reasons-invalid"]').doesNotExist();
      assert.dom('[data-test-id="detail-invalid"]').exists();

      assert.strictEqual(window.openKwargs, undefined);
    });

    test('valid form without detail', async function (assert) {
      await prepare(this, assert);

      await click('[data-test-id="spam-checkbox"]');
      assert.dom('[data-test-id="spam-checkbox"]').isChecked();
      assert.dom('[data-test-id="detail-input"]').hasValue('');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').doesNotExist();
      assert.dom('[data-test-id="reasons-invalid"]').doesNotExist();
      assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

      let body = `I'm reporting the https://crates.io/crates/nanomsg crate because:

- [x] it contains spam
- [ ] it is name-squatting (reserving a crate name without content)
- [ ] it is abusive or otherwise harmful
- [ ] it contains a vulnerability (please try to contact the crate author first)
- [ ] it is violating the usage policy in some other way (please specify below)

Additional details:


`;
      let subject = `The "nanomsg" crate`;
      let address = 'help@crates.io';
      let mailto = `mailto:${address}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
      assert.true(!!window.openKwargs);
      assert.strictEqual(window.openKwargs.url, mailto);
      assert.strictEqual(window.openKwargs.target, '_self');
    });

    test('valid form with required detail', async function (assert) {
      await prepare(this, assert);

      await click('[data-test-id="spam-checkbox"]');
      assert.dom('[data-test-id="spam-checkbox"]').isChecked();
      await click('[data-test-id="other-checkbox"]');
      assert.dom('[data-test-id="other-checkbox"]').isChecked();
      await fillIn('[data-test-id="detail-input"]', 'test detail');
      assert.dom('[data-test-id="detail-input"]').hasValue('test detail');
      await click('[data-test-id="report-button"]');

      assert.dom('[data-test-id="crate-invalid"]').doesNotExist();
      assert.dom('[data-test-id="reasons-invalid"]').doesNotExist();
      assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

      let body = `I'm reporting the https://crates.io/crates/nanomsg crate because:

- [x] it contains spam
- [ ] it is name-squatting (reserving a crate name without content)
- [ ] it is abusive or otherwise harmful
- [ ] it contains a vulnerability (please try to contact the crate author first)
- [x] it is violating the usage policy in some other way (please specify below)

Additional details:

test detail
`;
      let subject = `The "nanomsg" crate`;
      let address = 'help@crates.io';
      let mailto = `mailto:${address}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
      assert.true(!!window.openKwargs);
      assert.strictEqual(window.openKwargs.url, mailto);
      assert.strictEqual(window.openKwargs.target, '_self');
    });
  });
});
