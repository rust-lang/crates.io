export function preCreateExtension(preCreate) {
  return {
    name: 'preCreate',
    extend(collection) {
      collection.__counter = 0;

      collection.__originalCreate = collection.create;
      collection.create = async function (attrs) {
        attrs = attrs ?? {};
        preCreate(attrs, ++collection.__counter);
        return await collection.__originalCreate(attrs);
      };
    },
  };
}
