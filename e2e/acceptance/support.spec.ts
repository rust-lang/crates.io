import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | support page', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate, num: '0.6.0' });

    // mock `window.open()`
    await page.addInitScript(() => {
      globalThis.open = (url, target, features) => {
        globalThis.openKwargs = { url, target, features };
        return { document: { write() {}, close() {} }, close() {} } as ReturnType<(typeof globalThis)['open']>;
      };
    });
  });

  test('shows an inquire list', async ({ page, percy, a11y }) => {
    await page.goto('/support');
    await expect(page).toHaveURL('/support');

    await expect(page.getByTestId('support-main-content').locator('section')).toHaveCount(1);
    await expect(page.getByTestId('inquire-list-section')).toBeVisible();
    const inquireList = page.getByTestId('inquire-list');
    await expect(inquireList).toBeVisible();
    await expect(inquireList.locator(page.getByRole('listitem'))).toHaveText(
      ['Report a crate that violates policies'].concat(['For all other cases: help@crates.io']),
    );

    await percy.snapshot();
    await a11y.audit();
  });

  test('shows an inquire list if given inquire is not supported', async ({ page }) => {
    await page.goto('/support?inquire=not-supported-inquire');
    await expect(page).toHaveURL('/support?inquire=not-supported-inquire');

    await expect(page.getByTestId('support-main-content').locator('section')).toHaveCount(1);
    await expect(page.getByTestId('inquire-list-section')).toBeVisible();
    const inquireList = page.getByTestId('inquire-list');
    await expect(inquireList).toBeVisible();
    await expect(inquireList.locator(page.getByRole('listitem'))).toHaveText(
      ['Report a crate that violates policies'].concat(['For all other cases: help@crates.io']),
    );
  });

  test.describe('reporting a crate from support page', () => {
    test.beforeEach(async ({ page, msw }) => {
      await page.goto('/support');
      await page.getByTestId('link-crate-violation').click();
      await expect(page).toHaveURL('/support?inquire=crate-violation');
    });

    test('show a report form', async ({ page, percy, a11y }) => {
      await expect(page.getByTestId('support-main-content').locator('section')).toHaveCount(1);
      await expect(page.getByTestId('crate-violation-section')).toBeVisible();
      await expect(page.getByTestId('fieldset-crate')).toBeVisible();
      await expect(page.getByTestId('fieldset-reasons')).toBeVisible();
      await expect(page.getByTestId('fieldset-detail')).toBeVisible();
      await expect(page.getByTestId('report-button')).toHaveText('Report to help@crates.io');

      await percy.snapshot();
      await a11y.audit();
    });

    test('empty form should shows errors', async ({ page }) => {
      await page.getByTestId('report-button').click();

      await expect(page.getByTestId('crate-invalid')).toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).toBeVisible();
      await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
    });

    test('empty crate should shows errors', async ({ page }) => {
      const crateInput = page.getByTestId('crate-input');
      await expect(crateInput).toHaveValue('');
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).toBeVisible();
      await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
    });

    test('other reason selected without given detail shows an error', async ({ page }) => {
      const crateInput = page.getByTestId('crate-input');
      await crateInput.fill('nanomsg');
      await expect(crateInput).toHaveValue('nanomsg');

      const spam = page.getByTestId('spam-checkbox');
      await spam.check();
      await expect(spam).toBeChecked();
      const other = page.getByTestId('other-checkbox');
      await other.check();
      await expect(other).toBeChecked();
      const detailInput = page.getByTestId('detail-input');
      await expect(detailInput).toHaveValue('');
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).not.toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).not.toBeVisible();
      await expect(page.getByTestId('detail-invalid')).toBeVisible();

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
    });

    test('valid form without detail', async ({ page }) => {
      const crateInput = page.getByTestId('crate-input');
      await crateInput.fill('nanomsg');
      await expect(crateInput).toHaveValue('nanomsg');

      const spam = page.getByTestId('spam-checkbox');
      await spam.check();
      await expect(spam).toBeChecked();
      const detailInput = page.getByTestId('detail-input');
      await expect(detailInput).toHaveValue('');

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).not.toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).not.toBeVisible();
      await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

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
      // wait for `window.open()` to be called
      await page.waitForFunction(() => !!globalThis.openKwargs);
      await page.waitForFunction(expect => globalThis.openKwargs.url === expect, mailto);
      await page.waitForFunction(expect => globalThis.openKwargs.target === expect, '_self');
    });

    test('valid form with required detail', async ({ page }) => {
      const crateInput = page.getByTestId('crate-input');
      await crateInput.fill('nanomsg');
      await expect(crateInput).toHaveValue('nanomsg');

      const spam = page.getByTestId('spam-checkbox');
      await spam.check();
      await expect(spam).toBeChecked();
      const other = page.getByTestId('other-checkbox');
      await other.check();
      await expect(other).toBeChecked();
      const detailInput = page.getByTestId('detail-input');
      await detailInput.fill('test detail');
      await expect(detailInput).toHaveValue('test detail');

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).not.toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).not.toBeVisible();
      await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

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
      // wait for `window.open()` to be called
      await page.waitForFunction(() => !!globalThis.openKwargs);
      await page.waitForFunction(expect => globalThis.openKwargs.url === expect, mailto);
      await page.waitForFunction(expect => globalThis.openKwargs.target === expect, '_self');
    });
  });

  test.describe('reporting a crate from crate page', () => {
    test.beforeEach(async ({ page, msw }) => {
      let user = await msw.db.user.create({});
      await msw.authenticateAs(user);
      await page.goto('/crates/nanomsg');
      await page.getByTestId('link-crate-report').click();
      await expect(page).toHaveURL('/support?crate=nanomsg&inquire=crate-violation');
      await expect(page.getByTestId('crate-input')).toHaveValue('nanomsg');
    });

    test('empty crate should shows errors', async ({ page }) => {
      const crateInput = page.getByTestId('crate-input');
      await crateInput.fill('');
      await expect(crateInput).toHaveValue('');
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).toBeVisible();
      await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
    });

    test('other reason selected without given detail shows an error', async ({ page }) => {
      const spam = page.getByTestId('spam-checkbox');
      await spam.check();
      await expect(spam).toBeChecked();
      const other = page.getByTestId('other-checkbox');
      await other.check();
      await expect(other).toBeChecked();
      const detailInput = page.getByTestId('detail-input');
      await expect(detailInput).toHaveValue('');
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).not.toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).not.toBeVisible();
      await expect(page.getByTestId('detail-invalid')).toBeVisible();

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
    });

    test('valid form without detail', async ({ page }) => {
      const spam = page.getByTestId('spam-checkbox');
      await spam.check();
      await expect(spam).toBeChecked();
      const detailInput = page.getByTestId('detail-input');
      await expect(detailInput).toHaveValue('');

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).not.toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).not.toBeVisible();
      await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

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
      // wait for `window.open()` to be called
      await page.waitForFunction(() => !!globalThis.openKwargs);
      await page.waitForFunction(expect => globalThis.openKwargs.url === expect, mailto);
      await page.waitForFunction(expect => globalThis.openKwargs.target === expect, '_self');
    });

    test('valid form with required detail', async ({ page }) => {
      const spam = page.getByTestId('spam-checkbox');
      await spam.check();
      await expect(spam).toBeChecked();
      const other = page.getByTestId('other-checkbox');
      await other.check();
      await expect(other).toBeChecked();
      const detailInput = page.getByTestId('detail-input');
      await detailInput.fill('test detail');
      await expect(detailInput).toHaveValue('test detail');

      await page.waitForFunction(() => globalThis.openKwargs === undefined);
      const reportButton = page.getByTestId('report-button');
      await reportButton.click();

      await expect(page.getByTestId('crate-invalid')).not.toBeVisible();
      await expect(page.getByTestId('reasons-invalid')).not.toBeVisible();
      await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

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
      // wait for `window.open()` to be called
      await page.waitForFunction(() => !!globalThis.openKwargs);
      await page.waitForFunction(expect => globalThis.openKwargs.url === expect, mailto);
      await page.waitForFunction(expect => globalThis.openKwargs.target === expect, '_self');
    });
  });

  test('valid form with required detail', async ({ page }) => {
    await page.goto('/support');
    await page.getByTestId('link-crate-violation').click();
    await expect(page).toHaveURL('/support?inquire=crate-violation');

    const crateInput = page.getByTestId('crate-input');
    await crateInput.fill('nanomsg');
    await expect(crateInput).toHaveValue('nanomsg');
    const checkbox = page.getByTestId('malicious-code-checkbox');
    await checkbox.check();
    await expect(checkbox).toBeChecked();
    const detailInput = page.getByTestId('detail-input');
    await detailInput.fill('test detail');
    await expect(detailInput).toHaveValue('test detail');

    await page.waitForFunction(() => globalThis.openKwargs === undefined);
    const reportButton = page.getByTestId('report-button');
    await reportButton.click();

    await expect(page.getByTestId('crate-invalid')).not.toBeVisible();
    await expect(page.getByTestId('reasons-invalid')).not.toBeVisible();
    await expect(page.getByTestId('detail-invalid')).not.toBeVisible();

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
    // wait for `window.open()` to be called
    await page.waitForFunction(() => !!globalThis.openKwargs);
    await page.waitForFunction(expect => globalThis.openKwargs.url === expect, mailto);
    await page.waitForFunction(expect => globalThis.openKwargs.target === expect, '_self');
  });

  test('shows help text for vulnerability reports', async ({ page }) => {
    await page.goto('/support');
    await page.getByTestId('link-crate-violation').click();
    await expect(page).toHaveURL('/support?inquire=crate-violation');

    const crateInput = page.getByTestId('crate-input');
    await crateInput.fill('nanomsg');
    await expect(crateInput).toHaveValue('nanomsg');
    await expect(page.getByTestId('vulnerability-report')).not.toBeVisible();

    const checkbox = page.getByTestId('vulnerability-checkbox');
    await checkbox.check();
    await expect(checkbox).toBeChecked();
    await expect(page.getByTestId('vulnerability-report')).toBeVisible();
  });
});
