import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'crates-io/tests/helpers';

module('Component | RenderedHtml', function (hooks) {
  setupRenderingTest(hooks);

  test('renders HTML', async function (assert) {
    this.htmlContent = '<p>Hello <strong>world</strong>!</p>';

    await render(hbs`<RenderedHtml @html={{this.htmlContent}} />`);

    assert.dom('p').hasText('Hello world!');
    assert.dom('strong').hasText('world');
  });

  test('renders code blocks with syntax highlighting', async function (assert) {
    this.htmlContent = '<pre><code class="language-rust">fn main() {}</code></pre>';

    await render(hbs`<RenderedHtml @html={{this.htmlContent}} />`);

    assert.dom('pre').exists();
    assert.dom('code.language-rust').hasText('fn main() {}');
    assert.dom('.hljs-keyword').hasText('fn');
    assert.dom('.hljs-title').hasText('main');
  });

  test('renders mermaid diagrams', async function (assert) {
    let mermaidService = this.owner.lookup('service:mermaid');
    await mermaidService.loadTask.perform();

    this.htmlContent = '<pre><code class="language-mermaid">graph TD\n    A --> B</code></pre>';

    await render(hbs`<RenderedHtml @html={{this.htmlContent}} />`);

    assert.dom('pre').exists();
    assert.dom('code.language-mermaid svg.flowchart').exists();
    assert.dom('.nodeLabel').hasText('A');
  });
});
