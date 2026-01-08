const CATEGORY_PREFIX = 'category:';
const KEYWORD_PREFIX = 'keyword:';

export interface SearchParams {
  q: string;
  keyword?: string;
  all_keywords?: string;
  category?: string;
}

/**
 * Process a search query string and extract filters like `keyword:` and `category:`.
 *
 * Examples:
 * - "rust database" → { q: "rust database" }
 * - "rust keyword:async" → { q: "rust", keyword: "async" }
 * - "keyword:async keyword:web" → { q: "", all_keywords: "async web" }
 * - "rust category:algorithms" → { q: "rust", category: "algorithms" }
 */
export function processSearchQuery(query: string): SearchParams {
  let tokens = query.trim().split(/\s+/);

  let queries: string[] = [];
  let keywords: string[] = [];
  let category: string | null = null;

  for (let token of tokens) {
    if (token.startsWith(CATEGORY_PREFIX)) {
      let value = token.slice(CATEGORY_PREFIX.length).trim();
      if (value) {
        category = value;
      }
    } else if (token.startsWith(KEYWORD_PREFIX)) {
      let value = token.slice(KEYWORD_PREFIX.length).trim();
      if (value) {
        keywords.push(value);
      }
    } else {
      queries.push(token);
    }
  }

  let result: SearchParams = { q: queries.join(' ') };

  if (keywords.length === 1) {
    result.keyword = keywords[0];
  } else if (keywords.length !== 0) {
    result.all_keywords = keywords.join(' ');
  }

  if (category) {
    result.category = category;
  }

  return result;
}

/**
 * Check if the query contains multiple category: filters (which is not yet supported).
 */
export function hasMultiCategoryFilter(query: string): boolean {
  let tokens = query.trim().split(/\s+/);
  return tokens.filter(token => token.startsWith(CATEGORY_PREFIX)).length > 1;
}
