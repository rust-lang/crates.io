import { expect, test } from '@/e2e/helper';

const SOURCE_FILES = {
  'Cargo.toml': '[package]\nname = "serde"\n',
  'src/lib.rs': '// serde crate root\npub fn answer() -> u32 { 42 }\n',
  'src/de.rs': '// serde deserializer\npub struct Deserializer;\n',
  'docs/guide.md': '# Guide\n\nWelcome.\n',
  'examples/icon.png': `PNG${String.fromCodePoint(0)}binary data`,
};

test.describe('Acceptance | crate code viewer', { tag: '@acceptance' }, () => {
  test('redirects `/code` to the default file and renders it', async ({ page, msw, percy }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: SOURCE_FILES });

    await page.goto('/crates/serde/1.0.0/code');
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs');
    await expect(page.locator('[data-test-code-viewer]')).toContainText('// serde crate root');
    await expect(page.getByRole('treeitem', { name: 'lib.rs', selected: true })).toBeVisible();

    await percy.snapshot();
    await expect(page).toMatchAriaSnapshot({ name: 'aria.yml' });
  });

  test('redirects `/code` without a version to the default version', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: SOURCE_FILES });

    await page.goto('/crates/serde/code');
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs');
    await expect(page.locator('[data-test-code-viewer]')).toContainText('// serde crate root');
  });

  test('forwards the requested path when redirecting to the default version', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: SOURCE_FILES });

    await page.goto('/crates/serde/code/src/de.rs');
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/de.rs');
    await expect(page.locator('[data-test-code-viewer]')).toContainText('// serde deserializer');
  });

  test('redirects a directory to its first file', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: SOURCE_FILES });

    await page.goto('/crates/serde/1.0.0/code/docs');
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/docs/guide.md');
    await expect(page.locator('[data-test-code-viewer]')).toContainText('Welcome.');
  });

  test('navigates between files via the tree and browser history', async ({ page, msw }) => {
    let longBody = Array.from({ length: 300 }, (_, i) => `// line ${i}`).join('\n');
    let sourceFiles = {
      ...SOURCE_FILES,
      'src/long-a.rs': `// long file a\n${longBody}\n`,
      'src/long-b.rs': `// long file b\n${longBody}\n`,
    };

    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: sourceFiles });

    let viewer = page.locator('[data-test-code-viewer]');

    await page.goto('/crates/serde/1.0.0/code/src/lib.rs');
    await expect(viewer).toContainText('// serde crate root');

    await page.getByRole('treeitem', { name: 'de.rs' }).click();
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/de.rs');
    await expect(viewer).toContainText('// serde deserializer');

    await page.goBack();
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs');
    await expect(viewer).toContainText('// serde crate root');

    await page.getByRole('treeitem', { name: 'long-a.rs' }).click();
    await expect(viewer).toContainText('// long file a');
    await viewer.evaluate(el => {
      el.scrollTop = el.scrollHeight;
    });
    await expect.poll(() => viewer.evaluate(el => el.scrollTop)).toBeGreaterThan(0);

    await page.getByRole('treeitem', { name: 'long-b.rs' }).click();
    await expect(viewer).toContainText('// long file b');
    await expect.poll(() => viewer.evaluate(el => el.scrollTop)).toBe(0);
  });

  test('selects code lines from GitHub-style URL hashes', async ({ page, msw }) => {
    let sourceFiles = {
      ...SOURCE_FILES,
      'src/lib.rs': ['// line 1', '// line 2', '// line 3', '// line 4', '// line 5'].join('\n'),
    };

    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: sourceFiles });

    await page.goto('/crates/serde/1.0.0/code/src/lib.rs#L2-L4');
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs#L2-L4');
    await expect.poll(() => page.evaluate(() => globalThis.history.length)).toBe(2);

    let line = (number: number) => page.locator(`[data-line="${number}"]`).first();
    let expectSelectedLines = async (selectedLines: number[]) => {
      let selected = new Set(selectedLines);

      for (let number of [1, 2, 3, 4, 5]) {
        let expectation = expect(line(number));

        // eslint-disable-next-line unicorn/prefer-ternary
        if (selected.has(number)) {
          await expectation.toHaveAttribute('data-selected-line');
        } else {
          await expectation.not.toHaveAttribute('data-selected-line');
        }
      }
    };

    // Assert that lines 2-4 are selected.
    await expectSelectedLines([2, 3, 4]);

    // Mouse down on line 1 selects it.
    let line1 = await page.locator('[data-column-number="1"]').first().boundingBox();
    expect(line1).not.toBeNull();
    await page.mouse.move(line1!.x + line1!.width / 2, line1!.y + line1!.height / 2);
    await page.mouse.down();
    await expectSelectedLines([1]);
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs#L2-L4');
    await expect.poll(() => page.evaluate(() => globalThis.history.length)).toBe(2);

    // Moving the mouse to line 5 selects the 1-5 range.
    let line5 = await page.locator('[data-column-number="5"]').first().boundingBox();
    expect(line5).not.toBeNull();
    await page.mouse.move(line5!.x + line5!.width / 2, line5!.y + line5!.height / 2);
    await expectSelectedLines([1, 2, 3, 4, 5]);
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs#L2-L4');
    await expect.poll(() => page.evaluate(() => globalThis.history.length)).toBe(2);

    // Mouse up updates the URL hash.
    await page.mouse.up();
    await expectSelectedLines([1, 2, 3, 4, 5]);
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs#L1-L5');
    await expect.poll(() => page.evaluate(() => globalThis.history.length)).toBe(2);

    // Navigate to a different file.
    await page.getByRole('treeitem', { name: 'de.rs' }).click();
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/de.rs');
    await expect.poll(() => page.evaluate(() => globalThis.history.length)).toBe(3);
    await expect(page.locator('[data-test-code-viewer]')).toContainText('// serde deserializer');

    // Navigate back to the previous file and ensure that line 1-5 is selected.
    await page.goBack();
    await expect(page).toHaveURL('/crates/serde/1.0.0/code/src/lib.rs#L1-L5');
    await expectSelectedLines([1, 2, 3, 4, 5]);
    await expect(page.locator('[data-test-code-viewer]')).toContainText('// line 1');
  });

  test('shows a message for binary files', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: SOURCE_FILES });

    await page.goto('/crates/serde/1.0.0/code/examples/icon.png');
    await expect(page.locator('[data-test-binary-file]')).toBeVisible();
  });

  test('shows a not-available message when the archive is missing', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await page.goto('/crates/serde/1.0.0/code');
    await expect(page.locator('[data-test-archive-unavailable]')).toBeVisible();
  });

  test('shows an error for an unknown file path', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', source_files: SOURCE_FILES });

    await page.goto('/crates/serde/1.0.0/code/src/missing.rs');
    await expect(page.locator('[data-test-load-error]')).toBeVisible();
  });
});
