import DocumentTitle from "ember-document-title/mixins/document-title";

DocumentTitle.reopen({
    "titleSpecificityIncreases": false,
    "titleDivider": "-"
});

export default {
  name: "document-title-config",
  initialize() {}
};
