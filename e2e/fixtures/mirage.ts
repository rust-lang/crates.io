import { Page } from '@playwright/test';
import { Registry, Server as BaseServer, Request } from 'miragejs';
import { HandlerOptions, RouteHandler, ServerConfig } from 'miragejs/server';
import { CONFIG_KEY, HOOK_KEY } from '@/mirage/config';

const HOOK_MAPPING = {
  config: CONFIG_KEY,
  hook: HOOK_KEY,
} as const;
const DEFAULT_MIRAGE_CONFIG = { environment: 'test' } as const;

export class MiragePage {
  constructor(public readonly page: Page) {
    this.page = page;
  }

  async config(options: ServerConfig<Models, Factories> = DEFAULT_MIRAGE_CONFIG) {
    await this._config({ options, force: true });
  }

  private async _config({ options, force }: { options?: ServerConfig<Models, Factories>; force: boolean }) {
    await this.page.addInitScript(
      ({ key, options, force }) => {
        if (force || !window[Symbol.for(`${key}`)]) {
          window[Symbol.for(`${key}`)] = options;
        }
      },
      { key: HOOK_MAPPING.config, options: { ...DEFAULT_MIRAGE_CONFIG, ...options }, force },
    );
  }

  async addHook(hook: HookFn | HookScript) {
    let fn = String((k: string, h: HookFn) => {
      let key = Symbol.for(`${k}`);
      window[key] = (window[key] || []).concat(h);
    });
    await this.page.addInitScript(`(${fn})('${HOOK_MAPPING.hook}', ${hook.toString()});`);
  }

  private async addHelpers() {
    await this.page.addInitScript(() => {
      globalThis.authenticateAs = function (user) {
        globalThis.server.create('mirage-session', { user });
        globalThis.localStorage.setItem('isLoggedIn', '1');
      };
    });
    // Use default options only if no other options are explicitly provided
    await this._config({ force: false });
  }

  async setup() {
    await this.addHelpers();
  }
}

interface Server extends BaseServer<Registry<Models, Factories>> {
  get<Response extends AnyResponse>(path: string, response: Response, status?: number): void;
  get<Response extends AnyResponse>(
    path: string,
    handler?: RouteHandler<Registry<Models, Factories>, Response>,
    options?: HandlerOptions,
  ): void;
  put<Response extends AnyResponse>(path: string, response: Response, status?: number): void;
  put<Response extends AnyResponse>(
    path: string,
    handler?: RouteHandler<Registry<Models, Factories>, Response>,
    options?: HandlerOptions,
  ): void;
  patch<Response extends AnyResponse>(path: string, response: Response, status?: number): void;
  patch<Response extends AnyResponse>(
    path: string,
    handler?: RouteHandler<Registry<Models, Factories>, Response>,
    options?: HandlerOptions,
  ): void;
  delete<Response extends AnyResponse>(path: string, response: Response, status?: number): void;
  delete<Response extends AnyResponse>(
    path: string,
    handler?: RouteHandler<Registry<Models, Factories>, Response>,
    options?: HandlerOptions,
  ): void;
  del<Response extends AnyResponse>(path: string, response: Response, status?: number): void;
  del<Response extends AnyResponse>(
    path: string,
    handler?: RouteHandler<Registry<Models, Factories>, Response>,
    options?: HandlerOptions,
  ): void;
  _config: ServerConfig<Models, Factories>;
  pretender: PretenderSever;
}

interface PretenderSever extends BasePretenderServer {
  handledRequests: Request[];
}

type HookFn = (server: Server) => void;
type HookScript = Exclude<Parameters<Page['addInitScript']>[0], Function>;
type BasePretenderServer = BaseServer['pretender'];

declare global {
  var server: Server;
  // TODO: Improve typing
  function authenticateAs(user): void;
}

import { default as ApiTokenModel } from '@/mirage/models/api-token';
import { default as CategorySlugModel } from '@/mirage/models/category-slug';
import { default as CategoryModel } from '@/mirage/models/category';
import { default as CrateOwnerInvitationModel } from '@/mirage/models/crate-owner-invitation';
import { default as CrateOwnershipModel } from '@/mirage/models/crate-ownership';
import { default as CrateModel } from '@/mirage/models/crate';
import { default as DependencyModel } from '@/mirage/models/dependency';
import { default as KeywordModel } from '@/mirage/models/keyword';
import { default as MirageSessionModel } from '@/mirage/models/mirage-session';
import { default as OwnedCrateModel } from '@/mirage/models/owned-crate';
import { default as TeamModel } from '@/mirage/models/team';
import { default as UserModel } from '@/mirage/models/user';
import { default as VersionDownloadModel } from '@/mirage/models/version-download';
import { default as VersionModel } from '@/mirage/models/version';

import { default as ApiTokenFactory } from '@/mirage/factories/api-token';
import { default as CategoryFactory } from '@/mirage/factories/category';
import { default as CrateOwnerInvitationFactory } from '@/mirage/factories/crate-owner-invitation';
import { default as CrateOwnershipFactory } from '@/mirage/factories/crate-ownership';
import { default as CrateFactory } from '@/mirage/factories/crate';
import { default as DependencyFactory } from '@/mirage/factories/dependency';
import { default as KeywordFactory } from '@/mirage/factories/keyword';
import { default as MirageSessionFactory } from '@/mirage/factories/mirage-session';
import { default as TeamFactory } from '@/mirage/factories/team';
import { default as UserFactory } from '@/mirage/factories/user';
import { default as VersionDownloadFactory } from '@/mirage/factories/version-download';
import { default as VersionFactory } from '@/mirage/factories/version';
import { AnyResponse } from 'miragejs/-types';

const ModelsCamel = {
  apiToken: ApiTokenModel,
  categorySlug: CategorySlugModel,
  category: CategoryModel,
  crateOwnerInvitation: CrateOwnerInvitationModel,
  crateOwnership: CrateOwnershipModel,
  crate: CrateModel,
  dependency: DependencyModel,
  keyword: KeywordModel,
  mirageSession: MirageSessionModel,
  ownedCrate: OwnedCrateModel,
  team: TeamModel,
  user: UserModel,
  versionDownload: VersionDownloadModel,
  version: VersionModel,
};

type Models = typeof ModelsCamel & KebabKeys<typeof ModelsCamel>;

const FactoriesCamel = {
  apiToken: ApiTokenFactory,
  category: CategoryFactory,
  crateOwnerInvitation: CrateOwnerInvitationFactory,
  crateOwnership: CrateOwnershipFactory,
  crate: CrateFactory,
  dependency: DependencyFactory,
  keyword: KeywordFactory,
  mirageSession: MirageSessionFactory,
  team: TeamFactory,
  user: UserFactory,
  versionDownload: VersionDownloadFactory,
  version: VersionFactory,
};

type Factories = typeof FactoriesCamel;

// Taken from https://stackoverflow.com/a/66140779
type Kebab<T extends string, A extends string = ''> = T extends `${infer F}${infer R}`
  ? Kebab<R, `${A}${F extends Lowercase<F> ? '' : '-'}${Lowercase<F>}`>
  : A;
type KebabKeys<T> = { [K in keyof T as K extends string ? Kebab<K> : K]: T[K] };
