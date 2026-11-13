import type { operations } from '@crates-io/api-client';
import type { BlogPost } from '#lib/utils/blog.ts';

import { browser } from '$app/env';
import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';
import { loadBlogFeed, parseBlogPosts } from '#lib/utils/blog.ts';

type SummaryResponse = operations['get_summary']['responses']['200']['content']['application/json'];

let cachedSummary: SummaryResponse | undefined;
let cachedBlogPosts: BlogPost[] | undefined;

/**
 * Load function to fetch summary data and the latest blog posts for the page.
 *
 * Both are cached on the client side to avoid redundant network requests
 * and are streamed to the page instead of waiting for the entire data to be fetched
 * before rendering.
 */
export async function load({ fetch }) {
  let client = createClient({ fetch });

  let blogPosts = loadBlogPosts(fetch);
  // Handles the rejection right away, because the page only subscribes to the promise after rendering.
  blogPosts.catch(reportBlogPostsError);

  return { summary: loadSummary(client), blogPosts };
}

async function loadSummary(client: ReturnType<typeof createClient>): Promise<SummaryResponse> {
  if (browser && cachedSummary) {
    return cachedSummary;
  }

  let summary = await loadApi(client.GET('/api/v1/summary'), { message: 'Failed to load summary data' });

  if (browser) {
    cachedSummary = summary;
  }

  return summary;
}

async function loadBlogPosts(fetch: typeof globalThis.fetch): Promise<BlogPost[]> {
  if (browser && cachedBlogPosts) {
    return cachedBlogPosts;
  }

  let blogPosts = parseBlogPosts(await loadBlogFeed(fetch));

  if (browser) {
    cachedBlogPosts = blogPosts;
  }

  return blogPosts;
}

function reportBlogPostsError(error: unknown) {
  // `fetch()` rejects with a `TypeError` on network errors, which are not actionable.
  if (!(error instanceof TypeError)) {
    console.error(error);
  }
}
