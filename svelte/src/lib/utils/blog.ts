const BLOG_FEED_URL = 'https://blog.rust-lang.org/tags/crates-io/feed.xml';

/**
 * Loads the Atom feed of crates.io related posts from the Rust blog.
 *
 * @throws Error if the request fails or the response is not well-formed XML.
 */
export async function loadBlogFeed(fetch: typeof globalThis.fetch): Promise<Document> {
  let response = await fetch(BLOG_FEED_URL, { priority: 'low' });
  if (!response.ok) {
    throw new Error('Failed to load blog feed');
  }

  let feed = new DOMParser().parseFromString(await response.text(), 'application/xml');
  if (feed.querySelector('parsererror')) {
    throw new Error('Failed to parse blog feed');
  }

  return feed;
}
