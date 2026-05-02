import { describe, expect, test } from 'vitest';

import stripTestSelectors from './strip-test-selectors.js';

interface PreprocessResult {
  code: string;
  map?: object;
}

function run(source: string): PreprocessResult | undefined {
  let group = stripTestSelectors();
  let result = group.markup!({ content: source, filename: 'Test.svelte' });
  return result as PreprocessResult | undefined;
}

function transform(source: string): string {
  return run(source)?.code ?? source;
}

describe('strip-test-selectors', () => {
  describe('strips matching attributes', () => {
    test('boolean attribute', () => {
      expect(transform(`<div data-test-foo></div>`)).toMatchInlineSnapshot(`"<div ></div>"`);
    });

    test('multiple attributes on one element, mixed with other attributes', () => {
      expect(transform(`<div class="x" data-test-a id="y" data-test-b></div>`)).toMatchInlineSnapshot(
        `"<div class="x"  id="y" ></div>"`,
      );
    });

    test('multiple elements, multiple selectors each', () => {
      let input = [
        `<div data-test-a data-test-b>1</div>`,
        `<div data-test-c data-test-d>2</div>`,
        `<div data-test-e data-test-f>3</div>`,
      ].join('\n');

      expect(transform(input)).toMatchInlineSnapshot(`
        "<div  >1</div>
        <div  >2</div>
        <div  >3</div>"
      `);
    });

    test('static string value', () => {
      expect(transform(`<div data-test-foo="bar"></div>`)).toMatchInlineSnapshot(`"<div ></div>"`);
    });

    test('expression value', () => {
      expect(transform(`<div data-test-foo={someExpr}></div>`)).toMatchInlineSnapshot(`"<div ></div>"`);
    });

    test('quoted value with interpolation', () => {
      expect(transform(`<div data-test-foo="hello {name}"></div>`)).toMatchInlineSnapshot(`"<div ></div>"`);
    });

    test('data-testid exact match', () => {
      expect(transform(`<div data-testid="x"></div>`)).toMatchInlineSnapshot(`"<div ></div>"`);
    });

    test('component attribute', () => {
      expect(transform(`<PageHeader data-test-heading other={x}></PageHeader>`)).toMatchInlineSnapshot(
        `"<PageHeader  other={x}></PageHeader>"`,
      );
    });

    test('self-closing element', () => {
      expect(transform(`<input data-test-foo />`)).toMatchInlineSnapshot(`"<input  />"`);
    });
  });

  describe('preserves non-matching markup', () => {
    test('non-matching data-* attribute', () => {
      let input = `<div data-foo></div>`;
      expect(transform(input)).toBe(input);
    });

    test('data-test with no hyphen and no `id` suffix', () => {
      let input = `<div data-test></div>`;
      expect(transform(input)).toBe(input);
    });

    test('text content containing the literal', () => {
      let input = `<span>data-test-foo</span>`;
      expect(transform(input)).toBe(input);
    });

    test('CSS selector inside <style>', () => {
      let input = `<div></div>\n<style>[data-test-foo] { color: red; }</style>`;
      expect(transform(input)).toBe(input);
    });

    test('string literal inside <script>', () => {
      let input = `<script>let x = 'data-test-foo';</script>\n<div></div>`;
      expect(transform(input)).toBe(input);
    });

    test('spread attribute alone', () => {
      let input = `<div {...rest}></div>`;
      expect(transform(input)).toBe(input);
    });
  });

  describe('output shape', () => {
    test('returns code and map when changes are made', () => {
      let result = run(`<div data-test-foo></div>`);
      expect(result).toBeDefined();
      expect(result!.code).toBe(`<div ></div>`);
      expect(result!.map).toBeDefined();
      expect(typeof result!.map).toBe('object');
    });

    test('returns undefined when no changes are made', () => {
      let result = run(`<div class="x"></div>`);
      expect(result).toBeUndefined();
    });
  });
});
