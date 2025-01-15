import { factory as mswFactory } from '@mswjs/data';

/**
 * This function creates a new MSW database instance with the given models.
 *
 * This is a custom factory function that extends the default MSW factory
 * by adding support for a `preCreate()` function that is executed before
 * creating a new model and has access to the model attributes.
 */
export function factory(models) {
  // Extract `preCreate()` functions from the model definitions
  // and store them in a separate Map.
  let preCreateFns = new Map();
  for (let [modelName, modelDef] of Object.entries(models)) {
    if (modelDef.preCreate) {
      preCreateFns.set(modelName, modelDef.preCreate);
      delete modelDef.preCreate;
    }
  }

  // Create a new MSW database instance with the given models.
  let db = mswFactory(models);

  // Override the `create()` method of each model to apply
  // the `preCreate()` function before creating a new model.
  for (let [key, preCreate] of preCreateFns.entries()) {
    let modelApi = db[key];

    modelApi.mswCreate = modelApi.create;
    modelApi.create = function (attrs = {}) {
      preCreate(attrs);
      return modelApi.mswCreate(attrs);
    };
  }

  return db;
}
