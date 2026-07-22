import type { operations } from '@crates-io/api-client';

type Operation = keyof operations;
type Responses<Op extends Operation> = operations[Op]['responses'];

type JsonBody<Response> = Response extends { content: { 'application/json': infer Body } } ? Body : never;

type JsonSuccessStatus<Op extends Operation> = {
  [Status in keyof Responses<Op>]: `${Extract<Status, string | number>}` extends `2${string}`
    ? JsonBody<Responses<Op>[Status]> extends never
      ? never
      : Status
    : never;
}[keyof Responses<Op>];

type IsUnion<Type, Copy = Type> = Type extends Copy ? ([Copy] extends [Type] ? false : true) : never;

type OnlyStatus<Status> = [Status] extends [never] ? never : IsUnion<Status> extends false ? Status : never;

/**
 * The JSON body returned by a successful OpenAPI operation.
 *
 * `Status` can be omitted when the operation has exactly one successful JSON response.
 */
export type SuccessBody<
  Op extends Operation,
  Status extends JsonSuccessStatus<Op> = OnlyStatus<JsonSuccessStatus<Op>>,
> = Status extends keyof Responses<Op> ? JsonBody<Responses<Op>[Status]> : never;
