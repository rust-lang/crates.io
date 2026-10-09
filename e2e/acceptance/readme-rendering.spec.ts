import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

const README_HTML = `
<div class="markdown-alert markdown-alert-note">
<p class="markdown-alert-title">Note</p>
<p>Useful information that users should know, even when skimming content.</p>
</div>
<div class="markdown-alert markdown-alert-tip">
<p class="markdown-alert-title">Tip</p>
<p>Helpful advice for doing things better or more easily.</p>
</div>
<div class="markdown-alert markdown-alert-important">
<p class="markdown-alert-title">Important</p>
<p>Key information users need to know to achieve their goal.</p>
</div>
<div class="markdown-alert markdown-alert-warning">
<p class="markdown-alert-title">Warning</p>
<p>Urgent info that needs immediate user attention to avoid problems.</p>
</div>
<div class="markdown-alert markdown-alert-caution">
<p class="markdown-alert-title">Caution</p>
<p>Advises about risks or negative outcomes of certain actions.</p>
</div>

<div class="markdown-alert markdown-alert-note">
<p class="markdown-alert-title">Note</p>
<div class="markdown-alert markdown-alert-important">
<p class="markdown-alert-title">Important</p>
<div class="markdown-alert markdown-alert-caution">
<p class="markdown-alert-title">Caution</p>
<p>Rick roll</p>
<p>Never gonna give you up</p>
</div>
</div>
</div>

<div class="markdown-heading" dir="auto">
<h2 tabindex="-1" class="heading-element" dir="auto">Table of Contents</h2>
<a id="user-content-table-of-contents" class="anchor" aria-label="Permalink: Table of Contents" href="#table-of-contents"></a>
</div>
<ul dir="auto">
<li><a href="#serde-in-action">Serde in action</a></li>
<li><a href="#user-content-getting-help">Getting help</a></li>
<li><a href="#tests-without-prefix">Tests without prefix</a></li>
<li><a href="#%F0%9F%A6%80-is-all-we-need">🦀 is all we need</a></li>
</ul>

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
    <pre><code class="language-rust">struct Stack { inner: Vec&lt;u32&gt; }
impl Stack {
    delegate! {
        to self.inner {
            #[call(push)]
            pub fn add(&amp;mut self, value: u32);
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
<h2><a href="#tests-without-prefix" id="tests-without-prefix" rel="nofollow noopener noreferrer"></a>Tests without prefix</h2>
<h2><a href="#%F0%9F%A6%80-is-all-we-need" id="🦀-is-all-we-need" rel="nofollow noopener noreferrer"></a>🦀 is all we need</h2>

<a href="/crates/syn#resource">Go to syn resource</a>

<h3 align="center">
  <a>
    <img width="1000" height="200" alt="Banner with Logo" src="https://static.rerun.io/d0f5443d4803cac65c73fcc064936c09f5e7f208_rerun_banner.png" />
  </a>
</h3>

<a href="https://www.jetbrains.com/?from=rust-base64"><img src="https://raw.githubusercontent.com/marshallpierce/rust-base64/069bf7067b949f5c0a92b6ceb82492920502f2c2/icon_CLion.svg" height="40px"/></a>

<img src="https://raw.githubusercontent.com/scylladb/scylla-rust-driver/ff6415dcb3f2cb52bd5312c2ae99b063895a5f28/assets/monster%2Brust.png" height="150" align="right">

This is a client-side driver for [ScyllaDB] written in pure Rust with a fully async API using [Tokio].
Although optimized for ScyllaDB, the driver is also compatible with [Apache Cassandra®].
`;

test.describe('Acceptance | README rendering', { tag: '@acceptance' }, () => {
  test('it works', async ({ page, msw, percy }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', readme: README_HTML });

    await page.goto('/crates/serde');
    let readme = page.locator('[data-test-readme]');
    await expect(readme).toBeVisible();
    await expect(readme.locator('ul > li')).toHaveCount(11);
    await expect(readme.locator('pre > code.language-rust:has(span.line)')).toHaveCount(2);
    await expect(readme.locator('pre > code.language-mermaid svg.flowchart')).toBeVisible();

    await percy.snapshot();
    await expect(page).toMatchAriaSnapshot({ name: 'aria.yml' });
  });

  test('it shows a fallback if no readme is available', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await page.goto('/crates/serde');
    await expect(page.locator('[data-test-no-readme]')).toBeVisible();
  });

  test('it shows an error message and retry button if loading fails', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', readme: 'foo' });

    // Simulate a server error when fetching the README
    msw.worker.use(
      http.get('https://static.crates.io/readmes/serde/serde-1.0.0.html', () => HttpResponse.html('', { status: 500 })),
    );

    await page.goto('/crates/serde');
    await expect(page.locator('[data-test-readme-error]')).toBeVisible();
    await expect(page.locator('[data-test-retry-button]')).toBeVisible();

    await msw.worker.resetHandlers();

    await page.click('[data-test-retry-button]');
    await expect(page.locator('[data-test-readme]')).toHaveText('foo');
  });

  test('it scrolls to the anchor spot once rendered', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', readme: README_HTML });

    // Unprefixed URI fragment that matches the prefixed one
    // `/crates/serde#serde-in-action` -> `#user-content-serde-in-action`
    {
      await page.goto('/crates/serde#serde-in-action');
      let readme = page.locator('[data-test-readme]');
      await expect(readme).toBeVisible();
      let serdeInAction = page.getByRole('heading', { name: 'Serde in action' });
      await expect(serdeInAction).toBeInViewport();
    }

    // Exact match for the full prefixed URI fragment
    // `/crates/serde#user-content-getting-help` -> `#user-content-getting-help`
    {
      await page.goto('/crates/serde#user-content-getting-help');
      let readme = page.locator('[data-test-readme]');
      await expect(readme).toBeVisible();
      let gettingHelp = page.getByRole('heading', { name: 'Getting help' });
      await expect(gettingHelp).toBeInViewport();
    }

    // Exact match for an unprefixed URI fragment
    // `/crates/serde#tests-without-prefix` -> `#tests-without-prefix`
    {
      await page.goto('/crates/serde#tests-without-prefix');
      let readme = page.locator('[data-test-readme]');
      await expect(readme).toBeVisible();
      let testsWithoutPrefix = page.getByRole('heading', { name: 'Tests without prefix' });
      await expect(testsWithoutPrefix).toBeInViewport();
    }

    // Non-ASCII URI fragment
    // `/crates/serde#%F0%9F%A6%80-is-all-we-need` -> `#🦀-is-all-we-need`
    {
      await page.goto('/crates/serde#%F0%9F%A6%80-is-all-we-need');
      let readme = page.locator('[data-test-readme]');
      await expect(readme).toBeVisible();
      let rustIsAllWeNeed = page.getByRole('heading', { name: '🦀 is all we need' });
      await expect(rustIsAllWeNeed).toBeInViewport();
    }
  });

  test('it scrolls to the anchor spot when clicked', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0', readme: README_HTML });
    let crate2 = await msw.db.crate.create({ name: 'syn' });
    await msw.db.version.create({
      crate: crate2,
      num: '1.0.0',
      readme: `
<h1>syn</h1>
${README_HTML}
<h2><a href="#resource" id="resource" rel="nofollow noopener noreferrer"></a>Resource</h2>
`,
    });

    await page.goto('/crates/serde');
    let readme = page.locator('[data-test-readme]');
    await expect(readme).toBeVisible();

    // Click on `/crates/serde#serde-in-action` should scroll to `#user-content-anchor`
    await page.getByRole('link', { name: 'Serde in action' }).click();
    let serdeInAction = page.getByRole('heading', { name: 'Serde in action' });
    await expect(serdeInAction).toBeInViewport();

    // Click on `/crates/serde#user-content-getting-help` should scroll to
    // `#user-content-getting-help`
    await page.getByRole('link', { name: 'Getting help' }).click();
    let gettingHelp = page.getByRole('heading', { name: 'Getting help' });
    await expect(gettingHelp).toBeInViewport();

    // Click on `/crates/serde#tests-without-prefix` should scroll to `#tests-without-prefix`
    await page.getByRole('link', { name: 'Tests without prefix' }).click();
    let testsWithoutPrefix = page.getByRole('heading', { name: 'Tests without prefix' });
    await expect(testsWithoutPrefix).toBeInViewport();

    // Click on `/crates/serde#%F0%9F%A6%80-is-all-we-need` should scroll to `#🦀-is-all-we-need`
    await page.getByRole('link', { name: '🦀 is all we need' }).click();
    let rustIsAllWeNeed = page.getByRole('heading', { name: '🦀 is all we need' });
    await expect(rustIsAllWeNeed).toBeInViewport();

    // Click on `/crates/syn#resource` on page `/crates/serde` should work
    await page.getByRole('link', { name: 'Go to syn resource' }).click();
    await page.waitForURL('/crates/syn#resource');
    let resource = page.getByRole('heading', { name: 'Resource' });
    await expect(resource).toBeInViewport();
  });
});
