import { http, HttpResponse } from 'msw';

interface FeedEntry {
  title: string;
  url: string;
  published: string;
}

const entries: FeedEntry[] = [
  {
    title: 'crates.io: development update',
    url: 'https://blog.rust-lang.org/2025/07/11/crates-io-development-update/',
    published: '2025-07-11T00:00:00+00:00',
  },
  {
    title: 'crates.io: Help test our new web frontend',
    url: 'https://blog.rust-lang.org/inside-rust/2025/06/20/crates-io-new-frontend/',
    published: '2025-06-20T00:00:00+00:00',
  },
  {
    title: 'crates.io: Malicious crates faster_log, async_println, and a few others with rather long names',
    url: 'https://blog.rust-lang.org/2025/05/24/crates-io-malicious-crates/',
    published: '2025-05-24T00:00:00+00:00',
  },
  {
    title: 'crates.io: Usage policy &amp; trusted publishing',
    url: 'https://blog.rust-lang.org/2025/03/18/crates-io-usage-policy/',
    published: '2025-03-18T00:00:00+00:00',
  },
];

function renderEntry({ title, url, published }: FeedEntry) {
  return `
    <entry>
        <title>${title}</title>
        <link rel="alternate" href="${url}" type="text/html" title="${title}" />
        <published>${published}</published>
        <updated>${published}</updated>
        <id>${url}</id>
        <content type="html" xml:base="${url}">&lt;p&gt;Lorem ipsum dolor sit amet.&lt;/p&gt;</content>
        <author>
            <name>The crates.io team</name>
        </author>
    </entry>`;
}

const feed = `<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="en">
    <title>Posts tagged "crates.io" | Rust Blog</title>
    <link href="https://blog.rust-lang.org/tags/crates-io/feed.xml" rel="self" type="application/atom+xml" />
    <link href="https://blog.rust-lang.org/" rel="alternate" type="text/html" />
    <id>https://blog.rust-lang.org/tags/crates-io/feed.xml</id>
    <updated>${entries[0]!.published}</updated>
${entries.map(entry => renderEntry(entry)).join('\n')}
</feed>
`;

export default [
  http.get('https://blog.rust-lang.org/tags/crates-io/feed.xml', () =>
    HttpResponse.xml(feed, { headers: { 'Content-Type': 'application/xml' } }),
  ),
];
