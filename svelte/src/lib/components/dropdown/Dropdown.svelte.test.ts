import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import TestDropdown from './TestDropdown.svelte';

describe('Dropdown', () => {
  it('toggles content visibility when trigger is clicked', async () => {
    await render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });
    let content = page.getByText('Menu content');

    await expect.element(trigger).toBeVisible();
    await expect.element(content).not.toBeInTheDocument();

    await trigger.click();
    await expect.element(content).toBeVisible();

    await trigger.click();
    await expect.element(content).not.toBeInTheDocument();
  });

  it('closes when clicking outside', async () => {
    await render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });
    let content = page.getByText('Menu content');

    await trigger.click();
    await expect.element(content).toBeVisible();

    await page.getByTestId('outside').click();
    await expect.element(content).not.toBeInTheDocument();
  });

  it('closes when Escape key is pressed', async () => {
    await render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });
    let content = page.getByText('Menu content');

    await trigger.click();
    await expect.element(content).toBeVisible();

    await userEvent.keyboard('{Escape}');
    await expect.element(content).not.toBeInTheDocument();
  });

  it('does not close when Escape key is pressed while focus is outside', async () => {
    await render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });
    let content = page.getByText('Menu content');

    await trigger.click();
    await expect.element(content).toBeVisible();

    trigger.element().blur();
    await userEvent.keyboard('{Escape}');
    await expect.element(content).toBeVisible();
  });

  it('does not return focus to the trigger when closed via Escape from a menu item', async () => {
    await render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Menu Dropdown' });
    await trigger.click();

    let item = page.getByRole('link', { name: 'Item 1' });
    item.element().focus();
    await expect.element(item).toHaveFocus();

    await userEvent.keyboard('{Escape}');
    await expect.element(page.getByRole('menu')).not.toBeInTheDocument();
    await expect.element(trigger).not.toHaveFocus();
  });

  describe('positioning', () => {
    function rects(triggerName: string) {
      let trigger = page.getByRole('button', { name: triggerName }).element();
      let content = document.getElementById(trigger.getAttribute('aria-controls')!)!;
      return { trigger: trigger.getBoundingClientRect(), content: content.getBoundingClientRect() };
    }

    it('places the content directly below the trigger and at least as wide', async () => {
      await render(TestDropdown);
      await page.getByRole('button', { name: 'Wide Trigger' }).click();

      // content is directly below the trigger
      await expect.poll(() => rects('Wide Trigger').content.top - rects('Wide Trigger').trigger.bottom).toBeCloseTo(0);

      // content is as wide as the trigger
      let { trigger, content } = rects('Wide Trigger');
      expect(content.left).toBeCloseTo(trigger.left);
      expect(content.width).toBeCloseTo(trigger.width);
    });
  });

  it('closes when another dropdown opens', async () => {
    await render(TestDropdown);
    // open the lower trigger first so the upper menu can't cover it
    let first = page.getByRole('button', { name: 'No Arrow' });
    let second = page.getByRole('button', { name: 'Open Menu' });

    await first.click();
    await expect.element(page.getByText('Content', { exact: true })).toBeVisible();
    await expect.element(first).toHaveAttribute('aria-expanded', 'true');

    await second.click();
    await expect.element(page.getByText('Content', { exact: true })).not.toBeInTheDocument();
    await expect.element(first).toHaveAttribute('aria-expanded', 'false');
    await expect.element(page.getByText('Menu content')).toBeVisible();
    await expect.element(second).toHaveAttribute('aria-expanded', 'true');
  });

  it('sets correct ARIA attributes on trigger', async () => {
    await render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });

    expect(trigger.element().getAttribute('id')).toMatch(/^dropdown-trigger-/);
    expect(trigger.element().getAttribute('aria-controls')).toMatch(/^dropdown-content-/);

    await expect.element(trigger).toHaveAttribute('aria-expanded', 'false');
    await trigger.click();
    await expect.element(trigger).toHaveAttribute('aria-expanded', 'true');

    await userEvent.keyboard('{Escape}');
    await expect.element(trigger).toHaveAttribute('aria-expanded', 'false');
  });

  it('shows arrow indicator by default, hides when hideArrow is true', async () => {
    await render(TestDropdown);

    let withArrow = page.getByRole('button', { name: 'Open Menu' });
    expect(withArrow.element().querySelector('.arrow')).not.toBeNull();

    let withoutArrow = page.getByRole('button', { name: 'No Arrow' });
    expect(withoutArrow.element().querySelector('.arrow')).toBeNull();
  });
});

describe('Dropdown.Menu', () => {
  it('renders with correct ARIA roles and attributes', async () => {
    await render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Menu Dropdown' });
    await trigger.click();

    let menu = page.getByRole('menu');
    await expect.element(menu).toBeVisible();

    let menuItems = page.getByRole('menuitem');
    await expect.element(menuItems.first()).toBeVisible();

    let triggerId = trigger.element().getAttribute('id');
    await expect.element(menu).toHaveAttribute('aria-labelledby', triggerId!);
  });
});
