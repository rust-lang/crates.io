import { expect, test } from '@/e2e/helper';
import { Response } from 'miragejs';

const README_HTML = `
<p><strong>Serde is a framework for <em>ser</em>ializing and <em>de</em>serializing Rust data structures efficiently and generically.</strong></p>
<hr>
<p>You may be looking for:</p>
<ul>
<li><a href="https://serde.rs/" rel="nofollow noopener noreferrer">An overview of Serde</a></li>
<li><a href="https://serde.rs/#data-formats" rel="nofollow noopener noreferrer">Data formats supported by Serde</a></li>
<li><a href="https://serde.rs/derive.html" rel="nofollow noopener noreferrer">Setting up <code>#[derive(Serialize, Deserialize)]</code></a></li>
<li><a href="https://serde.rs/examples.html" rel="nofollow noopener noreferrer">Examples</a></li>
<li><a href="https://docs.serde.rs/serde/" rel="nofollow noopener noreferrer">API documentation</a></li>
<li><a href="https://github.com/serde-rs/serde/releases" rel="nofollow noopener noreferrer">Release notes</a></li>
</ul>
<h2><a href="#serde-in-action" id="user-content-serde-in-action" rel="nofollow noopener noreferrer"></a>Serde in action</h2>
<pre><code class="language-rust">use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let point = Point { x: 1, y: 2 };

    // Convert the Point to a JSON string.
    let serialized = serde_json::to_string(&amp;point).unwrap();

    // Prints serialized = {"x":1,"y":2}
    println!("serialized = {}", serialized);

    // Convert the JSON string back to a Point.
    let deserialized: Point = serde_json::from_str(&amp;serialized).unwrap();

    // Prints deserialized = Point { x: 1, y: 2 }
    println!("deserialized = {:?}", deserialized);
}
</code></pre>
<h2><a href="#getting-help" id="user-content-getting-help" rel="nofollow noopener noreferrer"></a>Getting help</h2>
<p>Serde is one of the most widely used Rust libraries so any place that Rustaceans
congregate will be able to help you out. For chat, consider trying the
<a href="https://discord.com/channels/273534239310479360/274215136414400513" rel="nofollow noopener noreferrer">#general</a> or <a href="https://discord.com/channels/273534239310479360/273541522815713281" rel="nofollow noopener noreferrer">#beginners</a> channels of the unofficial community Discord, the
<a href="https://discord.com/channels/442252698964721669/443150878111694848" rel="nofollow noopener noreferrer">#rust-usage</a> channel of the official Rust Project Discord, or the
<a href="https://rust-lang.zulipchat.com/#narrow/stream/122651-general" rel="nofollow noopener noreferrer">#general</a> stream in Zulip. For asynchronous, consider the <a href="https://stackoverflow.com/questions/tagged/rust" rel="nofollow noopener noreferrer">[rust] tag
on StackOverflow</a>, the <a href="https://www.reddit.com/r/rust" rel="nofollow noopener noreferrer">/r/rust</a> subreddit which has a pinned
weekly easy questions post, or the Rust <a href="https://users.rust-lang.org" rel="nofollow noopener noreferrer">Discourse forum</a>. It's
acceptable to file a support issue in this repo but they tend not to get as many
eyes as any of the above and may get closed without a response after some time.</p>

<p>Hello World!<sup><a href="#user-content-fn-1" id="user-content-fnref-1" rel="nofollow noopener noreferrer">1</a></sup></p>

<pre><code class="language-mermaid">
graph TD;
    A-->B;
    A-->C;
    B-->D;
    C-->D;
</code></pre>

<ul>
  <li>
    <p>Delegate to a method with a different name</p>
    <pre><code class="language-rust hljs" data-highlighted="yes"><span class="hljs-keyword">struct</span> <span class="hljs-title class_">Stack</span> { inner: <span class="hljs-type">Vec</span>&lt;<span class="hljs-type">u32</span>&gt; }
<span class="hljs-keyword">impl</span> <span class="hljs-title class_">Stack</span> {
    delegate! {
        to <span class="hljs-keyword">self</span>.inner {
            <span class="hljs-meta">#[call(push)]</span>
            <span class="hljs-keyword">pub</span> <span class="hljs-keyword">fn</span> <span class="hljs-title function_">add</span>(&amp;<span class="hljs-keyword">mut</span> <span class="hljs-keyword">self</span>, value: <span class="hljs-type">u32</span>);
        }
    }
}
</code></pre>
  </li>
</ul>

<section class="footnotes">
<ol>
<li id="user-content-fn-1">
<p>Hello Ferris, actually! <a href="#user-content-fnref-1" rel="nofollow noopener noreferrer">↩</a></p>
</li>
</ol>
</section>
`;

test.describe('Acceptance | README rendering', { tag: '@acceptance' }, () => {
  test('it works', async ({ page, mirage, percy }) => {
    await page.addInitScript(readmeHTML => {
      globalThis.readmeHTML = readmeHTML;
    }, README_HTML);
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'serde' });
      server.create('version', { crate, num: '1.0.0', readme: globalThis.readmeHTML });
    });

    await page.goto('/crates/serde');
    const readme = page.locator('[data-test-readme]');
    await expect(readme).toBeVisible();
    await expect(readme.locator('ul > li')).toHaveCount(7);
    await expect(readme.locator('pre > code.language-rust.hljs')).toHaveCount(2);
    await expect(readme.locator('pre > code.language-mermaid svg')).toBeVisible();

    await percy.snapshot();
  });

  test('it shows a fallback if no readme is available', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'serde' });
      server.create('version', { crate, num: '1.0.0' });
    });

    await page.goto('/crates/serde');
    await expect(page.locator('[data-test-no-readme]')).toBeVisible();
  });

  test('it shows an error message and retry button if loading fails', async ({ page, mirage }) => {
    await page.exposeBinding('resp200', () => new Response(200, { 'Content-Type': 'text/html' }, 'foo'));

    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'serde' });
      server.create('version', { crate, num: '1.0.0' });

      server.logging = true;
      // Simulate a server error when fetching the README
      server.get('/api/v1/crates/:name/:version/readme', {}, 500);
    });

    await page.goto('/crates/serde');
    await expect(page.locator('[data-test-readme-error]')).toBeVisible();
    await expect(page.locator('[data-test-retry-button]')).toBeVisible();

    await page.evaluate(() => {
      // Simulate a successful response when fetching the README
      server.get('/api/v1/crates/:name/:version/readme', {});
    });

    await page.click('[data-test-retry-button]');
    await expect(page.locator('[data-test-readme]')).toHaveText('{}');
  });
});
