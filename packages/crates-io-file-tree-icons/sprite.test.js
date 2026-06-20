import { expect, test } from 'vitest';

import { generateSpriteModule } from './generate.js';
import { iconNames } from './icons.js';

test('sprite.gen.js is up to date', async () => {
  let generated = generateSpriteModule(iconNames);
  await expect(generated).toMatchFileSnapshot('./sprite.gen.js');
});
