import { FactoryAPI as mswFactoryApi, ModelDictionary } from '@mswjs/data/lib/glossary';

export declare type FactoryAPI<Dictionary> = mswFactoryApi<Dictionary> & {
  reset(): void;
};

export declare function factory<Dictionary extends ModelDictionary>(dictionary: Dictionary): FactoryAPI<Dictionary>;
