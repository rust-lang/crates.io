import { modifier } from 'ember-modifier';

/**
 * This modifier updates the `media` attribute of the `source` elements
 * based on the user's color scheme preference.
 *
 * The code was adapted from https://larsmagnus.co/blog/how-to-make-images-react-to-light-and-dark-mode.
 */
export default modifier((element, [colorPreference]) => {
  let pictures = element.querySelectorAll('picture');

  pictures.forEach(picture => {
    let sources = picture.querySelectorAll(
      'source[media*="prefers-color-scheme"], source[data-media*="prefers-color-scheme"]',
    );

    sources.forEach(source => {
      // Preserve the source `media` as a data-attribute
      // to be able to switch between preferences
      if (source.media?.includes('prefers-color-scheme')) {
        source.dataset.media = source.media;
      }

      // If the source element `media` target is the `preference`,
      // override it to 'all' to show
      // or set it to 'none' to hide
      if (source.dataset.media.includes(colorPreference)) {
        source.media = 'all';
      } else if (source) {
        source.media = 'none';
      }
    });
  });
});
