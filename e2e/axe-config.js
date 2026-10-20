export default {
  rules: {
    'color-contrast': {
      enabled: false,
    },
    // Front page list items intentionally limit their accessible name to the
    // crate name and expose the remaining text as a description instead.
    'label-content-name-mismatch': {
      enabled: false,
    },
  },
};
