import { click, fillIn } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../axe-config';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | create report dialog', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);

  function prepare(context) {
    let server = context.server;
    let crate = server.create('crate', { name: 'nanomsg', newest_version: '0.6.0' });
    server.create('version', { crate, num: '0.6.0' });
  }

  test('display a report form in dialog', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg');
    await click('[data-test-report-button]');

    assert.dom('[data-test-dialog-content] [data-test-reasons-group]').exists();
    assert.dom('[data-test-dialog-content] [data-test-detail-group]').exists();
    assert.dom('[data-test-dialog-content] [data-test-cancel]').hasText('Cancel');
    assert.dom('[data-test-dialog-content] [data-test-report]').hasText('Report');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('empty reasons selected shows an error', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg');
    await click('[data-test-report-button]');

    await fillIn('[data-test-name]', 'test detail');
    await click('[data-test-dialog-content] [data-test-report]');
    assert.dom('[data-test-dialog-content] [data-test-reasons-group] [data-test-error]').exists();
    assert.dom('[data-test-dialog-content] [data-test-detail-group] [data-test-error]').doesNotExist();
  });

  test('other reason selected without given detail shows an error', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg');
    await click('[data-test-report-button]');

    await click('[data-test-dialog-content] [data-test-reason="spam"]');
    await click('[data-test-dialog-content] [data-test-reason="other"]');
    await click('[data-test-dialog-content] [data-test-report]');
    assert.dom('[data-test-dialog-content] [data-test-reasons-group] [data-test-error]').doesNotExist();
    assert.dom('[data-test-dialog-content] [data-test-detail-group] [data-test-error]').exists();
  });

  test('valid report form should compose a mail and open', async function (assert) {
    prepare(this);
    let fakeWindow = { document: { write() {}, close() {} }, close() {} };
    window.open = (url, target) => {
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
      assert.strictEqual(url, mailto);
      assert.strictEqual(target, '_self');
      return fakeWindow;
    };

    await visit('/crates/nanomsg');
    await click('[data-test-report-button]');

    await click('[data-test-dialog-content] [data-test-reason="spam"]');
    await click('[data-test-dialog-content] [data-test-reason="other"]');
    await fillIn('[data-test-name]', 'test detail');
    await click('[data-test-dialog-content] [data-test-report]');
    assert.dom('[data-test-dialog-content]').doesNotExist();
  });
});
