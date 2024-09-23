import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | crate report dialog', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });
    });
  });

  test('display a report form in dialog', async ({ page, percy, a11y }) => {
    await page.goto('/crates/nanomsg');
    await page.click('[data-test-report-button]');

    const dialogContent = page.locator('[data-test-dialog-content]');
    await expect(dialogContent.locator('[data-test-reasons-group]')).toBeVisible();
    await expect(dialogContent.locator('[data-test-detail-group]')).toBeVisible();
    await expect(dialogContent.locator('[data-test-cancel]')).toHaveText('Cancel');
    await expect(dialogContent.locator('[data-test-report]')).toHaveText('Report');

    await percy.snapshot();
    await a11y.audit();
  });

  test('empty reasons selected shows an error', async ({ page }) => {
    await page.goto('/crates/nanomsg/');
    await page.click('[data-test-report-button]');

    const dialogContent = page.locator('[data-test-dialog-content]');
    await dialogContent.locator('[data-test-report]').click();
    await expect(dialogContent.locator('[data-test-reasons-group] [data-test-error]')).toBeVisible();
    await expect(dialogContent.locator('[data-test-detail-group] [data-test-error]')).toHaveCount(0);
  });

  test('other reason selected without given detail shows an error', async ({ page }) => {
    await page.goto('/crates/nanomsg/');
    await page.click('[data-test-report-button]');

    const dialogContent = page.locator('[data-test-dialog-content]');
    await dialogContent.locator('[data-test-reason="spam"]').click();
    await dialogContent.locator('[data-test-reason="other"]').click();
    await dialogContent.locator('[data-test-report]').click();
    await expect(dialogContent.locator('[data-test-reasons-group] [data-test-error]')).toHaveCount(0);
    await expect(dialogContent.locator('[data-test-detail-group] [data-test-error]')).toBeVisible();
  });

  test('valid report form should compose a mail and open', async ({ page }) => {
    // mock `window.open()`
    await page.addInitScript(() => {
      globalThis.open = (url, target, features) => {
        globalThis.openKwargs = { url, target, features };
        return { document: { write() {}, close() {} }, close() {} } as ReturnType<(typeof globalThis)['open']>;
      };
    });

    await page.goto('/crates/nanomsg/');
    await page.click('[data-test-report-button]');

    const dialogContent = page.locator('[data-test-dialog-content]');
    await dialogContent.locator('[data-test-reason="spam"]').click();
    await dialogContent.locator('[data-test-reason="other"]').click();
    await dialogContent.locator('[data-test-detail]').fill('test detail');
    await dialogContent.locator('[data-test-report]').click();

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
    // wait for `window.open()` to be called
    await page.waitForFunction(expect => globalThis.openKwargs.url === expect, mailto);
    await page.waitForFunction(expect => globalThis.openKwargs.target === expect, '_self');
    await expect(dialogContent).not.toBeVisible();
  });
});
