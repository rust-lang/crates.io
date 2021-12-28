const KEYWORD_PREFIX = 'keyword:';
const KEYWORDS_PREFIX = 'keywords:';

/**
 * Process a search query string and extract filters like `keywords:`.
 *
 * @param {string} query
 * @return {{ q: string, keyword?: string, all_keywords?: string }}
 */
export function processSearchQuery(query) {
  let tokens = query.trim().split(/\s+/);

  let queries = [];
  let keywords = [];
  for (let token of tokens) {
    if (token.startsWith(KEYWORD_PREFIX)) {
      let value = token.slice(KEYWORD_PREFIX.length).trim();
      if (value) {
        keywords.push(value);
      }
    } else if (token.startsWith(KEYWORDS_PREFIX)) {
      keywords = token
        .slice(KEYWORDS_PREFIX.length)
        .split(',')
        .map(it => it.trim())
        .filter(Boolean);
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

  return result;
}
