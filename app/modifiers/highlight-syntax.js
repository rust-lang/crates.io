import { modifier } from 'ember-modifier';
import hljs from 'highlight.js/lib/core';
import 'highlight.js/styles/github.css';
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

// these aliases are registered for compatibility with the Prism.js language names
// that we used before.
hljs.registerAliases('clike', { languageName: 'c' });
hljs.registerAliases('markup', { languageName: 'xml' });

export default modifier((element, _, { selector }) => {
  let elements = selector ? element.querySelectorAll(selector) : [element];

  for (let element of elements) {
    // if the code block has no allowed language tag we use `no-highlight` to avoid highlighting
    let hasLanguageClass = [...element.classList].some(it => /^language-.+/.test(it));
    if (!hasLanguageClass) {
      element.classList.add('no-highlight');
    }

    hljs.highlightElement(element);
  }
});
