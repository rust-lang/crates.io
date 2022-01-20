export const CATEGORY_PREFIX = 'category:';
const KEYWORD_PREFIX = 'keyword:';

/**
 * Process a search query string and extract filters like `keyword:`.
 *
 * @param {string} query
 * @return {{ q: string, keyword?: string, all_keywords?: string, category?: string }}
 */
export function processSearchQuery(query) {
  let tokens = query.trim().split(/\s+/);

  let queries = [];
  let keywords = [];
  let category = null;
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

  let result = { q: queries.join(' ') };

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
