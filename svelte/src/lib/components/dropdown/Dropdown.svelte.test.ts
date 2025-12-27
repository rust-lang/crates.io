import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import TestDropdown from './TestDropdown.svelte';

describe('Dropdown', () => {
  it('toggles content visibility when trigger is clicked', async () => {
    render(TestDropdown);

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
    render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });
    let content = page.getByText('Menu content');

    await trigger.click();
    await expect.element(content).toBeVisible();

    await page.getByTestId('outside').click();
    await expect.element(content).not.toBeInTheDocument();
  });

  it('closes when Escape key is pressed', async () => {
    render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });
    let content = page.getByText('Menu content');

    await trigger.click();
    await expect.element(content).toBeVisible();

    await userEvent.keyboard('{Escape}');
    await expect.element(content).not.toBeInTheDocument();
  });

  it('sets correct ARIA attributes on trigger', async () => {
    render(TestDropdown);

    let trigger = page.getByRole('button', { name: 'Open Menu' });

    expect(trigger.element().getAttribute('id')).toMatch(/^dropdown-trigger-/);
    expect(trigger.element().getAttribute('aria-controls')).toMatch(/^dropdown-content-/);

    await expect.element(trigger).toHaveAttribute('aria-expanded', 'false');
    await trigger.click();
    await expect.element(trigger).toHaveAttribute('aria-expanded', 'true');
  });

  it('shows arrow indicator by default, hides when hideArrow is true', async () => {
    render(TestDropdown);

    let withArrow = page.getByRole('button', { name: 'Open Menu' });
    expect(withArrow.element().querySelector('.arrow')).not.toBeNull();

    let withoutArrow = page.getByRole('button', { name: 'No Arrow' });
    expect(withoutArrow.element().querySelector('.arrow')).toBeNull();
  });
});

describe('Dropdown.Menu', () => {
  it('renders with correct ARIA roles and attributes', async () => {
    render(TestDropdown);

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
