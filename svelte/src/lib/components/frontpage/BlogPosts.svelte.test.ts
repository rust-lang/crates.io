import type { BlogPost } from '#lib/utils/blog.ts';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import BlogPosts from './BlogPosts.svelte';

const TAG_URL = 'https://blog.rust-lang.org/tags/crates-io/';

function post(title: string, url: string, date: string, isInsideRust = false): BlogPost {
  return { title, url, published: new Date(date), isInsideRust };
}

const POSTS = [
  post('Development update', 'https://blog.rust-lang.org/2025/07/11/update/', '2025-07-11'),
  post('New frontend', 'https://blog.rust-lang.org/inside-rust/2025/06/20/frontend/', '2025-06-20', true),
  post('Malicious crates', 'https://blog.rust-lang.org/2025/05/24/malicious/', '2025-05-24'),
  post('Usage policy', 'https://blog.rust-lang.org/2025/03/18/policy/', '2025-03-18'),
];

const ERROR_MESSAGE = 'The latest posts could not be loaded. You can find them on the Rust blog.';

describe('BlogPosts', () => {
  it('links to all posts on the Rust blog', async () => {
    await render(BlogPosts, { state: { status: 'loading' } });

    let link = page.getByRole('link', { name: 'All posts on the Rust blog' });
    await expect.element(link).toHaveAttribute('href', TAG_URL);
  });

  it('renders placeholders while loading', async () => {
    await render(BlogPosts, { state: { status: 'loading' } });

    await expect.element(page.getByCSS('[data-test-blog-posts] ul')).toHaveAttribute('aria-busy', 'true');
    expect(page.getByCSS('[data-test-blog-post-placeholder]').elements()).toHaveLength(3);
    expect(page.getByCSS('[data-test-blog-post]').elements()).toHaveLength(0);
  });

  it('renders the three latest posts', async () => {
    await render(BlogPosts, { state: { status: 'loaded', posts: POSTS } });

    let rows = page.getByCSS('[data-test-blog-post]');
    expect(rows.elements()).toHaveLength(3);

    let first = rows.nth(0);
    await expect.element(first).toHaveAttribute('href', POSTS[0]!.url);
    await expect.element(first).toHaveAttribute('target', '_blank');
    await expect.element(first.getByCSS('[data-test-title]')).toHaveTextContent('Development update');
    expect(first.element().querySelector('[data-test-inside-rust-badge]')).toBeNull();

    let time = first.getByCSS('time');
    await expect.element(time).toHaveAttribute('datetime', '2025-07-11T00:00:00.000Z');
    await expect.element(time).toHaveTextContent('Jul 11, 2025');

    await expect.element(rows.nth(2).getByCSS('[data-test-title]')).toHaveTextContent('Malicious crates');
    await expect.element(page.getByText('Usage policy')).not.toBeInTheDocument();
  });

  it('marks Inside Rust posts', async () => {
    await render(BlogPosts, { state: { status: 'loaded', posts: POSTS } });

    let badges = page.getByCSS('[data-test-inside-rust-badge]');
    expect(badges.elements()).toHaveLength(1);
    await expect.element(badges).toHaveTextContent('Inside Rust');

    let second = page.getByCSS('[data-test-blog-post]').nth(1);
    await expect.element(second).toHaveAttribute('href', POSTS[1]!.url);
    expect(second.element().querySelector('[data-test-inside-rust-badge]')).not.toBeNull();
  });

  it('renders titles as plain text', async () => {
    let title = 'Use <b>bold</b> moves';
    await render(BlogPosts, { state: { status: 'loaded', posts: [post(title, POSTS[0]!.url, '2025-07-11')] } });

    await expect.element(page.getByCSS('[data-test-title]')).toHaveTextContent(title);
    expect(page.getByCSS('[data-test-blog-post] b').elements()).toHaveLength(0);
  });

  it('renders a fallback message with a link to the blog on error', async () => {
    await render(BlogPosts, { state: { status: 'error' } });

    let error = page.getByCSS('[data-test-blog-posts-error]');
    await expect.element(error).toHaveTextContent(ERROR_MESSAGE);
    await expect.element(error.getByRole('link', { name: 'Rust blog' })).toHaveAttribute('href', TAG_URL);
    expect(page.getByCSS('[data-test-blog-posts] ul').elements()).toHaveLength(0);
  });

  it('renders the fallback message if there are no posts', async () => {
    await render(BlogPosts, { state: { status: 'loaded', posts: [] } });

    await expect.element(page.getByCSS('[data-test-blog-posts-error]')).toHaveTextContent(ERROR_MESSAGE);
    expect(page.getByCSS('[data-test-blog-posts] ul').elements()).toHaveLength(0);
  });
});
