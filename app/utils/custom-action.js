export async function customAction(record, { method, path, data }) {
  let modelClass = record.constructor;
  let modelName = modelClass.modelName;
  let adapter = record.store.adapterFor(modelName);

  let requestType = 'updateRecord';
  let baseUrl = adapter.buildURL(modelName, record.id, null, requestType);
  let url = path ? `${baseUrl}/${path}` : baseUrl;

  return await adapter.ajax(url, method, { data });
}
