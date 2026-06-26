import { expect, test } from '@/e2e/helper';
import { Locator } from '@playwright/test';

test.describe('Acceptance | crate navigation tabs', { tag: '@acceptance' }, () => {
  test('basic navigation between tabs works as expected', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate, num: '0.6.1' });

    let tabReadme = page.locator('[data-test-readme-tab] a');
    let tabCode = page.locator('[data-test-code-tab] a');
    let tabVersions = page.locator('[data-test-versions-tab] a');
    let tabDeps = page.locator('[data-test-deps-tab] a');
    let tabRevDeps = page.locator('[data-test-rev-deps-tab] a');
    let tabSettings = page.locator('[data-test-settings-tab] a');

    async function checkLinks(version: string = '') {
      let readmeLink = version ? `/crates/nanomsg/${version}` : '/crates/nanomsg';
      await expect(tabReadme).toHaveAttribute('href', readmeLink);
      let codeLink = version ? `/crates/nanomsg/${version}/code` : '/crates/nanomsg/code';
      await expect(tabCode).toHaveAttribute('href', codeLink);
      await expect(tabVersions).toHaveAttribute('href', '/crates/nanomsg/versions');
      let depsLink = version ? `/crates/nanomsg/${version}/dependencies` : '/crates/nanomsg/dependencies';
      await expect(tabDeps).toHaveAttribute('href', depsLink);
      await expect(tabRevDeps).toHaveAttribute('href', '/crates/nanomsg/reverse_dependencies');
    }

    async function checkTabActiveState(currentTab: Locator) {
      await expect(currentTab).toHaveAttribute('data-test-active');
      let otherTabs = [tabReadme, tabCode, tabVersions, tabDeps, tabRevDeps].filter(tab => tab !== currentTab);
      for (let tab of otherTabs) {
        await expect(tab).not.toHaveAttribute('data-test-active');
      }
    }

    // Readme
    let currentTab = tabReadme;
    await page.goto('/crates/nanomsg');
    await expect(page).toHaveURL('/crates/nanomsg');
    await checkLinks();
    await checkTabActiveState(currentTab);
    await expect(tabSettings).toHaveCount(0);

    // Code
    currentTab = tabCode;
    await currentTab.click();
    await expect(page).toHaveURL('/crates/nanomsg/0.6.1/code');
    await checkLinks('0.6.1');
    await checkTabActiveState(currentTab);
    await expect(tabSettings).toHaveCount(0);

    // Version
    currentTab = tabVersions;
    await currentTab.click();
    await expect(page).toHaveURL('/crates/nanomsg/versions');
    await checkLinks();
    await checkTabActiveState(currentTab);
    await expect(tabSettings).toHaveCount(0);

    // Deps
    currentTab = tabDeps;
    await currentTab.click();
    await expect(page).toHaveURL('/crates/nanomsg/0.6.1/dependencies');
    await checkLinks('0.6.1');
    await checkTabActiveState(currentTab);
    await expect(tabSettings).toHaveCount(0);

    // RevDeps
    currentTab = tabRevDeps;
    await currentTab.click();
    await expect(page).toHaveURL('/crates/nanomsg/reverse_dependencies');
    await checkLinks();
    await checkTabActiveState(currentTab);
    await expect(tabSettings).toHaveCount(0);
  });
});
