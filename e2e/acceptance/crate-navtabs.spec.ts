import { test, expect } from '@/e2e/helper';
import { Locator } from '@playwright/test';

test.describe('Acceptance | crate navigation tabs', { tag: '@acceptance' }, () => {
  test('basic navigation between tabs works as expected', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.1' });
    });

    const tabReadme = page.locator('[data-test-readme-tab] a');
    const tabVersions = page.locator('[data-test-versions-tab] a');
    const tabDeps = page.locator('[data-test-deps-tab] a');
    const tabRevDeps = page.locator('[data-test-rev-deps-tab] a');
    const tabSettings = page.locator('[data-test-settings-tab] a');

    async function checkLinks(version: string = '') {
      const readmeLink = version ? `/crates/nanomsg/${version}` : '/crates/nanomsg';
      await expect(tabReadme).toHaveAttribute('href', readmeLink);
      await expect(tabVersions).toHaveAttribute('href', '/crates/nanomsg/versions');
      const depsLink = version ? `/crates/nanomsg/${version}/dependencies` : '/crates/nanomsg/dependencies';
      await expect(tabDeps).toHaveAttribute('href', depsLink);
      await expect(tabRevDeps).toHaveAttribute('href', '/crates/nanomsg/reverse_dependencies');
    }

    async function checkTabActiveState(currentTab: Locator) {
      await expect(currentTab).toHaveAttribute('data-test-active');
      const otherTabs = [tabReadme, tabVersions, tabDeps, tabRevDeps].filter(tab => tab !== currentTab);
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
