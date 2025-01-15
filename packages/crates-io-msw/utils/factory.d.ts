import { FactoryAPI, ModelDictionary } from '@mswjs/data/lib/glossary';

export declare function factory<Dictionary extends ModelDictionary>(dictionary: Dictionary): FactoryAPI<Dictionary>;
