import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

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
`;

module('Acceptance | README rendering', function (hooks) {
  setupApplicationTest(hooks);

  test('it works', async function (assert) {
    let crate = this.server.create('crate', { name: 'serde' });
    this.server.create('version', { crate, num: '1.0.0', readme: README_HTML });

    await visit('/crates/serde');
    assert.dom('[data-test-readme]').exists();
    assert.dom('[data-test-readme] ul > li').exists({ count: 6 });
    assert.dom('[data-test-readme] pre > code.language-rust.hljs').exists();

    await percySnapshot(assert);
  });

  test('it shows a fallback if no readme is available', async function (assert) {
    let crate = this.server.create('crate', { name: 'serde' });
    this.server.create('version', { crate, num: '1.0.0' });

    await visit('/crates/serde');
    assert.dom('[data-test-no-readme]').exists();
  });
});
