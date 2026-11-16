import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import avatarPlaceholder from '#lib/assets/avatar-placeholder.svg';
import Avatar from './Avatar.svelte';

const JANE_DOE = {
  kind: 'user' as const,
  name: 'Jane Doe',
  login: 'janedoe',
  avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
};

const RUST_TEAM = {
  kind: 'team' as const,
  name: 'Rust Team',
  login: 'rust-lang',
  avatar: 'https://avatars.githubusercontent.com/u/5430905?v=4',
};

describe('Avatar', () => {
  it('renders a user avatar', async () => {
    await render(Avatar, { owner: JANE_DOE });

    let img = page.getByRole('img');
    await expect.element(img).toHaveAttribute('alt', 'Jane Doe (janedoe)');
    await expect.element(img).toHaveAttribute('title', 'Jane Doe');
    await expect
      .element(img)
      .toHaveAttribute(
        'srcset',
        'https://avatars.githubusercontent.com/u/1?v=4&s=32 32w, https://avatars.githubusercontent.com/u/1?v=4&s=64 64w, https://avatars.githubusercontent.com/u/1?v=4&s=128 128w, https://avatars.githubusercontent.com/u/1?v=4&s=256 256w',
      );
    await expect.element(img).toHaveAttribute('sizes', 'auto');
    await expect.element(img).toHaveAttribute('loading', 'lazy');
    await expect.element(img).toHaveStyle({ borderRadius: '50%' });
  });

  it('renders a team avatar with rounded corners', async () => {
    await render(Avatar, { owner: RUST_TEAM });

    let img = page.getByRole('img');
    await expect.element(img).toHaveAttribute('alt', 'Rust Team (rust-lang)');
    await expect.element(img).toHaveAttribute('title', 'Rust Team team');
    await expect.element(img).toHaveStyle({ borderRadius: '4px' });
  });

  it('omits the name from the alt text when it is missing', async () => {
    await render(Avatar, { owner: { ...JANE_DOE, name: null } });

    await expect.element(page.getByRole('img')).toHaveAttribute('alt', '(janedoe)');
  });

  it('renders a placeholder when the avatar is missing', async () => {
    await render(Avatar, { owner: { ...JANE_DOE, avatar: null } });

    let img = page.getByRole('img');
    await expect.element(img).toHaveAttribute('src', avatarPlaceholder);
    await expect.element(img).not.toHaveAttribute('srcset');
  });

  it('forwards attributes to the wrapper', async () => {
    await render(Avatar, { owner: JANE_DOE, class: 'custom', 'aria-hidden': 'true', 'data-test-avatar': '' });

    let wrapper = page.getByRole('img', { includeHidden: true }).element().parentElement!;
    expect(wrapper.classList).toContain('custom');
    expect(wrapper.getAttribute('aria-hidden')).toBe('true');
    expect(wrapper.dataset.testAvatar).toBe('');
  });

  it('takes its size from `--avatar-size` and scales the padding', async () => {
    await render(Avatar, { owner: JANE_DOE, style: '--avatar-size: 85px' });

    let img = page.getByRole('img').element();
    expect(img.parentElement!.getBoundingClientRect().width).toBe(85);
    expect(img.getBoundingClientRect().width).toBe(85);
    await expect.element(img).toHaveStyle({ paddingTop: '2.975px' });
  });

  it('keeps its size in a flex container that is too narrow', async () => {
    let { container } = await render(Avatar, { owner: JANE_DOE, style: '--avatar-size: 85px' });
    container.style.display = 'flex';
    container.style.width = '50px';
    container.append(Object.assign(document.createElement('div'), { style: 'width: 100px' }));

    let wrapper = page.getByRole('img').element().parentElement!;
    expect(wrapper.getBoundingClientRect().width).toBe(85);
  });

  it('keeps at least 1px of padding at small sizes', async () => {
    await render(Avatar, { owner: JANE_DOE, style: '--avatar-size: 22px' });

    await expect.element(page.getByRole('img')).toHaveStyle({ paddingTop: '1px' });
  });

  it('takes its shadow from `--avatar-shadow`', async () => {
    await render(Avatar, { owner: JANE_DOE, style: '--avatar-shadow: 0 1px 1px 0 red' });

    await expect.element(page.getByRole('img')).toHaveStyle({ boxShadow: 'rgb(255, 0, 0) 0px 1px 1px 0px' });
  });

  it('loads an image that matches the rendered size', async () => {
    // A served file with a query that no other test loads, because Chrome prefers already cached candidates.
    let avatar = '/assets/og-image.png?sizes-auto';
    await render(Avatar, { owner: { ...JANE_DOE, avatar }, style: '--avatar-size: 32px' });

    let img = page.getByRole('img').element() as HTMLImageElement;
    await expect.poll(() => img.currentSrc).toMatch(/&s=32$/);
  });
});
