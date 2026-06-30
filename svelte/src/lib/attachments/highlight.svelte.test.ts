import { describe, expect, it } from 'vitest';

import { highlightSyntax } from './highlight';

function codeBlock(language: string | undefined, source: string): HTMLElement {
  let code = document.createElement('code');
  if (language) {
    code.classList.add(`language-${language}`);
  }
  code.textContent = source;
  return code;
}

describe('highlightSyntax', () => {
  it('highlights Rust', async () => {
    let source = 'fn main() {\n    println!("hello");\n}';
    let code = codeBlock('rust', source);

    highlightSyntax()(code);

    // shiki's `<pre><code>` wrapper is stripped and the tokens carry inline
    // `light-dark()` colors.
    await expect.poll(() => code.innerHTML).toContain('light-dark(');
    expect(code.innerHTML).toMatchInlineSnapshot(`
      "<span class="line"><span style="color:light-dark(#D73A49, #F97583)">fn</span><span style="color:light-dark(#6F42C1, #B392F0)"> main</span><span style="color:light-dark(#24292E, #E1E4E8)">() {</span></span>
      <span class="line"><span style="color:light-dark(#6F42C1, #B392F0)">    println!</span><span style="color:light-dark(#24292E, #E1E4E8)">(</span><span style="color:light-dark(#032F62, #9ECBFF)">"hello"</span><span style="color:light-dark(#24292E, #E1E4E8)">);</span></span>
      <span class="line"><span style="color:light-dark(#24292E, #E1E4E8)">}</span></span>"
    `);

    // The highlighted markup preserves the original source text verbatim.
    expect(code.textContent).toBe(source);
  });

  it('lazily loads and highlights another language (Go)', async () => {
    let code = codeBlock('go', 'package main');

    highlightSyntax()(code);

    await expect.poll(() => code.innerHTML).toContain('light-dark(');
  });

  it('resolves shiki built-in aliases (protobuf)', async () => {
    let code = codeBlock('protobuf', 'message Foo { int32 id = 1; }');

    highlightSyntax()(code);

    await expect.poll(() => code.innerHTML).toContain('light-dark(');
  });

  it('resolves Prism aliases (clike, markup)', async () => {
    let clike = codeBlock('clike', 'int main() { return 0; }');
    highlightSyntax()(clike);
    await expect.poll(() => clike.innerHTML).toContain('light-dark(');

    let markup = codeBlock('markup', '<a href="example">link</a>');
    highlightSyntax()(markup);
    await expect.poll(() => markup.innerHTML).toContain('light-dark(');
  });

  it('leaves code blocks without a known language untouched', async () => {
    let container = document.createElement('div');
    container.innerHTML =
      '<pre><code class="language-unknownlang">some text</code></pre>' +
      '<pre><code class="language-go">package main</code></pre>';
    let [unhandled, go] = container.querySelectorAll('code');
    let original = unhandled.innerHTML;

    highlightSyntax(undefined, 'pre > code')(container);

    // The lazily loaded Go block highlights through the same async pipeline, so
    // once it is done an unknown language would have been highlighted too.
    await expect.poll(() => go.innerHTML).toContain('light-dark(');
    expect(unhandled.innerHTML).toBe(original);
    expect(unhandled.textContent).toBe('some text');
  });

  it('skips mermaid blocks excluded by the selector', async () => {
    let container = document.createElement('div');
    container.innerHTML =
      '<pre><code class="language-mermaid">graph TD;</code></pre>' +
      '<pre><code class="language-go">package main</code></pre>';
    let [mermaid, go] = container.querySelectorAll('code');
    let original = mermaid.innerHTML;

    highlightSyntax(undefined, 'pre > code:not(.language-mermaid)')(container);

    // The lazily loaded Go block highlights through the same async pipeline, so
    // once it is done the excluded mermaid block would have been too.
    await expect.poll(() => go.innerHTML).toContain('light-dark(');
    expect(mermaid.innerHTML).toBe(original);
  });

  it('highlights blocks matched by a selector', async () => {
    let container = document.createElement('div');
    container.innerHTML = '<pre><code class="language-rust">let x = 1;</code></pre>';

    highlightSyntax(undefined, 'pre > code:not(.language-mermaid)')(container);

    let code = container.querySelector('code')!;
    await expect.poll(() => code.innerHTML).toContain('light-dark(');
  });
});
