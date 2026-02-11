import hljs from 'highlight.js/lib/core';
import bash from 'highlight.js/lib/languages/bash';
import c from 'highlight.js/lib/languages/c';
import cpp from 'highlight.js/lib/languages/cpp';
import csharp from 'highlight.js/lib/languages/csharp';
import glsl from 'highlight.js/lib/languages/glsl';
import go from 'highlight.js/lib/languages/go';
import ini from 'highlight.js/lib/languages/ini';
import javascript from 'highlight.js/lib/languages/javascript';
import json from 'highlight.js/lib/languages/json';
import protobuf from 'highlight.js/lib/languages/protobuf';
import ruby from 'highlight.js/lib/languages/ruby';
import rust from 'highlight.js/lib/languages/rust';
import scss from 'highlight.js/lib/languages/scss';
import sql from 'highlight.js/lib/languages/sql';
import xml from 'highlight.js/lib/languages/xml';
import yaml from 'highlight.js/lib/languages/yaml';

hljs.registerLanguage('bash', bash);
hljs.registerLanguage('c', c);
hljs.registerLanguage('cpp', cpp);
hljs.registerLanguage('csharp', csharp);
hljs.registerLanguage('glsl', glsl);
hljs.registerLanguage('go', go);
hljs.registerLanguage('ini', ini);
hljs.registerLanguage('javascript', javascript);
hljs.registerLanguage('json', json);
hljs.registerLanguage('protobuf', protobuf);
hljs.registerLanguage('ruby', ruby);
hljs.registerLanguage('rust', rust);
hljs.registerLanguage('scss', scss);
hljs.registerLanguage('sql', sql);
hljs.registerLanguage('xml', xml);
hljs.registerLanguage('yaml', yaml);

// Aliases for compatibility with Prism.js language names used previously
hljs.registerAliases('clike', { languageName: 'c' });
hljs.registerAliases('markup', { languageName: 'xml' });

// Common aliases
hljs.registerAliases('rs', { languageName: 'rust' });

/**
 * Attachment that applies syntax highlighting to code blocks using highlight.js.
 */
export function highlightSyntax(html?: string, selector?: string) {
  return (element: Element) => {
    // Ensure that the attachment is re-run when `html` changes
    void html;

    let elements = selector ? element.querySelectorAll(selector) : [element];

    for (let el of elements) {
      // If the code block has no language tag, use `no-highlight` to skip it
      let hasLanguageClass = [...el.classList].some(it => /^language-.+/.test(it));
      if (!hasLanguageClass) {
        el.classList.add('no-highlight');
      }

      hljs.highlightElement(el as HTMLElement);
    }
  };
}
