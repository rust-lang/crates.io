import { base } from '$app/paths';

// Inlined as a base64 data URI so the icon needs no separate request and
// never breaks: search clients fetch and cache it once at install time, so a
// stable, self-contained value avoids stale/404 icon references across deploys.
import icon from '$lib/assets/cargo.png?w=64&format=png&quality=80&inline&imagetools';

export const prerender = true;

export function GET({ url }) {
  /* eslint-disable unicorn/prefer-https -- OpenSearch XML namespace identifier, not a fetchable URL */
  let body = `<?xml version="1.0" encoding="utf-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
    <ShortName>crates.io</ShortName>
    <Description>Search for crates in the official Rust package registry</Description>
    <Image width="64" height="64">${icon}</Image>
    <Url type="text/html" method="get" template="${url.origin}${base}/search?q={searchTerms}"/>
</OpenSearchDescription>
`;
  /* eslint-enable unicorn/prefer-https */

  return new Response(body, {
    headers: { 'Content-Type': 'application/opensearchdescription+xml' },
  });
}
