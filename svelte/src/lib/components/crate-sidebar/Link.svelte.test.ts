import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import Link, { simplifyUrl } from './Link.svelte';

describe('simplifyUrl', () => {
  it('strips https:// prefix', () => {
    expect(simplifyUrl('https://rust-lang.org')).toBe('rust-lang.org');
  });

  it('strips www. prefix', () => {
    expect(simplifyUrl('https://www.rust-lang.org')).toBe('rust-lang.org');
  });

  it('does not strip http:// prefix', () => {
    expect(simplifyUrl('http://www.rust-lang.org')).toBe('http://www.rust-lang.org');
  });

  it('strips trailing slashes', () => {
    expect(simplifyUrl('https://www.rust-lang.org/')).toBe('rust-lang.org');
  });

  it('strips trailing .git from GitHub project URLs', () => {
    expect(simplifyUrl('https://github.com/rust-lang/crates.io.git')).toBe('github.com/rust-lang/crates.io');
  });

  it('does not strip trailing .git from non-GitHub URLs', () => {
    expect(simplifyUrl('https://foo.git/')).toBe('foo.git');
  });
});

describe('Link component', () => {
  it('renders title and link', async () => {
    render(Link, { title: 'Homepage', url: 'https://www.rust-lang.org' });

    await expect.element(page.getByCSS('[data-test-title]')).toHaveTextContent('Homepage');
    await expect.element(page.getByCSS('[data-test-icon]')).toHaveAttribute('data-test-icon', 'link');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveAttribute('href', 'https://www.rust-lang.org');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveTextContent('rust-lang.org');
  });

  it('renders GitHub icon for GitHub links', async () => {
    render(Link, { title: 'Repository', url: 'https://github.com/rust-lang/crates.io' });

    await expect.element(page.getByCSS('[data-test-icon]')).toHaveAttribute('data-test-icon', 'github');
    await expect
      .element(page.getByCSS('[data-test-link]'))
      .toHaveAttribute('href', 'https://github.com/rust-lang/crates.io');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveTextContent('github.com/rust-lang/crates.io');
  });

  it('renders docs.rs icon for docs.rs links', async () => {
    render(Link, { title: 'Documentation', url: 'https://docs.rs/tracing' });

    await expect.element(page.getByCSS('[data-test-icon]')).toHaveAttribute('data-test-icon', 'docs-rs');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveAttribute('href', 'https://docs.rs/tracing');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveTextContent('docs.rs/tracing');
  });

  it('renders GitLab icon for GitLab links', async () => {
    render(Link, { title: 'Repository', url: 'https://gitlab.com/example/project' });

    await expect.element(page.getByCSS('[data-test-icon]')).toHaveAttribute('data-test-icon', 'gitlab');
  });

  it('renders Codeberg icon for Codeberg links', async () => {
    render(Link, { title: 'Repository', url: 'https://codeberg.org/example/project' });

    await expect.element(page.getByCSS('[data-test-icon]')).toHaveAttribute('data-test-icon', 'codeberg');
  });

  it('does not shorten HTTP links', async () => {
    render(Link, { title: 'Homepage', url: 'http://www.rust-lang.org' });

    await expect.element(page.getByCSS('[data-test-link]')).toHaveAttribute('href', 'http://www.rust-lang.org');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveTextContent('http://www.rust-lang.org');
  });

  it('strips trailing slashes', async () => {
    render(Link, { title: 'Homepage', url: 'https://www.rust-lang.org/' });

    await expect.element(page.getByCSS('[data-test-link]')).toHaveAttribute('href', 'https://www.rust-lang.org/');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveTextContent('rust-lang.org');
  });

  it('strips trailing .git from GitHub project URLs', async () => {
    render(Link, { title: 'Repository', url: 'https://github.com/rust-lang/crates.io.git' });

    await expect
      .element(page.getByCSS('[data-test-link]'))
      .toHaveAttribute('href', 'https://github.com/rust-lang/crates.io.git');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveTextContent('github.com/rust-lang/crates.io');
  });

  it('does not strip trailing .git from non-GitHub URLs', async () => {
    render(Link, { title: 'Homepage', url: 'https://foo.git/' });

    await expect.element(page.getByCSS('[data-test-link]')).toHaveAttribute('href', 'https://foo.git/');
    await expect.element(page.getByCSS('[data-test-link]')).toHaveTextContent('foo.git');
  });
});
