import { click, currentURL, fillIn, findAll, getSettledState, waitFor } from '@ember/test-helpers';
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
  setupWindowMock(hooks);

  hooks.beforeEach(async function () {
    let crate = await this.db.crate.create({ name: 'nanomsg' });
    await this.db.version.create({ crate, num: '0.6.0' });

    window.open = (url, target, features) => {
      window.openKwargs = { url, target, features };
      return { document: { write() {}, close() {} }, close() {} };
    };
  });

  async function prepare(context) {
    let user = await context.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });
    await context.authenticateAs(user);
  }

  test('is not available if not logged in', async function (assert) {
    await visit('/support');
    assert.strictEqual(currentURL(), '/support');

    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('shows an inquire list', async function (assert) {
    await prepare(this);

    await visit('/support');
    assert.strictEqual(currentURL(), '/support');

    assert.dom('[data-test-id="support-main-content"] section').exists({ count: 1 });
    assert.dom('[data-test-id="inquire-list-section"]').exists();
    assert.dom('[data-test-id="inquire-list"]').exists();
    let listitem = findAll('[data-test-id="inquire-list"] li');
    assert.deepEqual(
      listitem.map(item => item.textContent.trim()),
      ['Report a crate that violates policies'].concat([
        `For all other cases:
              help@crates.io`,
      ]),
    );

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('shows an inquire list if given inquire is not supported', async function (assert) {
    await prepare(this);

    await visit('/support?inquire=not-supported-inquire');
    assert.strictEqual(currentURL(), '/support?inquire=not-supported-inquire');

    assert.dom('[data-test-id="support-main-content"] section').exists({ count: 1 });
    assert.dom('[data-test-id="inquire-list-section"]').exists();
    assert.dom('[data-test-id="inquire-list"]').exists();
    let listitem = findAll('[data-test-id="inquire-list"] li');
    assert.deepEqual(
      listitem.map(item => item.textContent.trim()),
      ['Report a crate that violates policies'].concat([
        `For all other cases:
              help@crates.io`,
      ]),
    );
  });

  module('reporting a crate from support page', function () {
    async function prepare(context, assert) {
      let user = await context.db.user.create({
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      });
      await context.authenticateAs(user);

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
      assert.dom('[data-test-id="report-button"]').hasText('Report to help@crates.io');

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
- [ ] it contains malicious code
- [ ] it contains a vulnerability
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
- [ ] it contains malicious code
- [ ] it contains a vulnerability
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
    async function prepare(context, assert) {
      let user = await context.db.user.create({
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      });
      await context.authenticateAs(user);

      await visit('/crates/nanomsg');
      assert.strictEqual(currentURL(), '/crates/nanomsg');

      try {
        await waitFor('[data-test-id="link-crate-report"]');
      } catch (error) {
        console.error(error);
        console.log(getSettledState());
        // display DOM tree for debugging
        let walker = document.createTreeWalker(
          document.querySelector('main'),
          NodeFilter.SHOW_ELEMENT + NodeFilter.SHOW_TEXT,
        );
        while (walker.nextNode()) {
          let current = walker.currentNode;
          if (current.nodeName === '#text') {
            let text = current.textContent.trim();
            if (text) {
              console.log(current.textContent, { current });
            }
          } else if (current.tagName && current.tagName !== 'path') {
            console.log(
              current.tagName,
              [...(current.attributes ?? [])].map(({ value, name }) => `${name}=${value}`).join(','),
            );
          }
        }
        throw error;
      }

      await click('[data-test-id="link-crate-report"]');
      assert.strictEqual(currentURL(), '/support?crate=nanomsg&inquire=crate-violation');
      assert.dom('[data-test-id="crate-input"]').hasValue('nanomsg');
    }

    test('is not available if not logged in', async function (assert) {
      await visit('/crates/nanomsg');
      assert.strictEqual(currentURL(), '/crates/nanomsg');

      assert.dom('[data-test-id="link-crate-report"]').doesNotExist();
    });

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
- [ ] it contains malicious code
- [ ] it contains a vulnerability
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
- [ ] it contains malicious code
- [ ] it contains a vulnerability
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

  test('malicious code reports are sent to security@rust-lang.org too', async function (assert) {
    await prepare(this);

    await visit('/support');
    await click('[data-test-id="link-crate-violation"]');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');

    await fillIn('[data-test-id="crate-input"]', 'nanomsg');
    assert.dom('[data-test-id="crate-input"]').hasValue('nanomsg');
    await click('[data-test-id="malicious-code-checkbox"]');
    assert.dom('[data-test-id="malicious-code-checkbox"]').isChecked();
    await fillIn('[data-test-id="detail-input"]', 'test detail');
    assert.dom('[data-test-id="detail-input"]').hasValue('test detail');
    await click('[data-test-id="report-button"]');

    assert.dom('[data-test-id="crate-invalid"]').doesNotExist();
    assert.dom('[data-test-id="reasons-invalid"]').doesNotExist();
    assert.dom('[data-test-id="detail-invalid"]').doesNotExist();

    let body = `I'm reporting the https://crates.io/crates/nanomsg crate because:

- [ ] it contains spam
- [ ] it is name-squatting (reserving a crate name without content)
- [ ] it is abusive or otherwise harmful
- [x] it contains malicious code
- [ ] it contains a vulnerability
- [ ] it is violating the usage policy in some other way (please specify below)

Additional details:

test detail
`;
    let subject = `[SECURITY] The "nanomsg" crate`;
    let addresses = 'help@crates.io,security@rust-lang.org';
    let mailto = `mailto:${addresses}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
    assert.true(!!window.openKwargs);
    assert.strictEqual(window.openKwargs.url, mailto);
    assert.strictEqual(window.openKwargs.target, '_self');
  });

  test('shows help text for vulnerability reports', async function (assert) {
    await prepare(this);

    await visit('/support');
    await click('[data-test-id="link-crate-violation"]');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');

    await fillIn('[data-test-id="crate-input"]', 'nanomsg');
    assert.dom('[data-test-id="crate-input"]').hasValue('nanomsg');
    assert.dom('[data-test-id="vulnerability-report"]').doesNotExist();

    await click('[data-test-id="vulnerability-checkbox"]');
    assert.dom('[data-test-id="vulnerability-checkbox"]').isChecked();
    assert.dom('[data-test-id="vulnerability-report"]').exists();
  });
});
