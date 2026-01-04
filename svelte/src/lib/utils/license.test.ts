import { describe, expect, it } from 'vitest';

import { parseLicense } from './license';

describe('parseLicense', () => {
  it.each([
    ['MIT', [{ isKeyword: false, link: 'https://choosealicense.com/licenses/mit', text: 'MIT' }]],
    [
      'MIT OR Apache-2.0',
      [
        { isKeyword: false, link: 'https://choosealicense.com/licenses/mit', text: 'MIT' },
        { isKeyword: true, link: undefined, text: 'OR' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/apache-2.0', text: 'Apache-2.0' },
      ],
    ],
    [
      'MIT/Apache-2.0',
      [
        { isKeyword: false, link: 'https://choosealicense.com/licenses/mit', text: 'MIT' },
        { isKeyword: true, link: undefined, text: 'OR' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/apache-2.0', text: 'Apache-2.0' },
      ],
    ],
    [
      'MIT/Apache-2.0/BSD-3-Clause',
      [
        { isKeyword: false, link: 'https://choosealicense.com/licenses/mit', text: 'MIT' },
        { isKeyword: true, link: undefined, text: 'OR' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/apache-2.0', text: 'Apache-2.0' },
        { isKeyword: true, link: undefined, text: 'OR' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/bsd-3-clause', text: 'BSD-3-Clause' },
      ],
    ],
    [
      'LGPL-2.1-only AND MIT AND BSD-2-Clause',
      [
        { isKeyword: false, link: 'https://spdx.org/licenses/LGPL-2.1-only.html', text: 'LGPL-2.1-only' },
        { isKeyword: true, link: undefined, text: 'AND' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/mit', text: 'MIT' },
        { isKeyword: true, link: undefined, text: 'AND' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/bsd-2-clause', text: 'BSD-2-Clause' },
      ],
    ],
    [
      'GPL-2.0-or-later WITH Bison-exception-2.2',
      [
        { isKeyword: false, link: 'https://spdx.org/licenses/GPL-2.0-or-later.html', text: 'GPL-2.0-or-later' },
        { isKeyword: true, link: undefined, text: 'WITH' },
        { isKeyword: false, link: 'https://spdx.org/licenses/Bison-exception-2.2.html', text: 'Bison-exception-2.2' },
      ],
    ],
    [
      'Unlicense OR MIT',
      [
        { isKeyword: false, link: 'https://choosealicense.com/licenses/unlicense', text: 'Unlicense' },
        { isKeyword: true, link: undefined, text: 'OR' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/mit', text: 'MIT' },
      ],
    ],
    [
      'A   OR  B',
      [
        { isKeyword: false, link: undefined, text: 'A' },
        { isKeyword: true, link: undefined, text: 'OR' },
        { isKeyword: false, link: undefined, text: 'B' },
      ],
    ],
    [
      '(Apache-2.0 OR MIT) AND BSD-3-Clause',
      [
        { isKeyword: true, link: undefined, text: '(' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/apache-2.0', text: 'Apache-2.0' },
        { isKeyword: true, link: undefined, text: 'OR' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/mit', text: 'MIT' },
        { isKeyword: true, link: undefined, text: ')' },
        { isKeyword: true, link: undefined, text: 'AND' },
        { isKeyword: false, link: 'https://choosealicense.com/licenses/bsd-3-clause', text: 'BSD-3-Clause' },
      ],
    ],
  ])('%s', (input, expected) => {
    expect(parseLicense(input)).toEqual(expected);
  });
});
