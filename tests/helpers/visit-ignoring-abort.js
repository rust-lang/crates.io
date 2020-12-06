import { visit as _visit, settled } from '@ember/test-helpers';

// see https://github.com/emberjs/ember-test-helpers/issues/332
export async function visit(url) {
  try {
    await _visit(url);
  } catch (error) {
    if (error.message !== 'TransitionAborted') {
      throw error;
    }
  }

  await settled();
}
