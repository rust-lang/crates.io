const BLOG_URL = 'https://blog.rust-lang.org/';
const BLOG_FEED_URL = `${BLOG_URL}tags/crates-io/feed.xml`;
const INSIDE_RUST_URL = `${BLOG_URL}inside-rust/`;

/**
 * A post from the Rust blog or its Inside Rust section.
 */
export interface BlogPost {
  title: string;
  url: string;
  published: Date;
  isInsideRust: boolean;
}

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

/**
 * Extracts the posts from a Rust blog Atom feed, newest first.
 *
 * Entries without a title, with an invalid publication date, or with a link
 * outside of the Rust blog are skipped.
 */
export function parseBlogPosts(feed: Document): BlogPost[] {
  let posts = [...feed.querySelectorAll(':root > entry')].flatMap(entry => {
    let title = entry.querySelector(':scope > title')?.textContent;
    let url = entry.querySelector(':scope > link[rel="alternate"]')?.getAttribute('href');
    let published = new Date(entry.querySelector(':scope > published')?.textContent ?? '');

    if (!title || !url?.startsWith(BLOG_URL) || Number.isNaN(published.getTime())) {
      return [];
    }

    return [{ title, url, published, isInsideRust: url.startsWith(INSIDE_RUST_URL) }];
  });

  return posts.toSorted((a, b) => b.published.getTime() - a.published.getTime());
}
