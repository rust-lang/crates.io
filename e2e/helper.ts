import { default as percySnapshot_ } from '@percy/playwright';
import { Page, TestInfo } from '@playwright/test';
import { Server, Registry } from 'miragejs';
import { ServerConfig } from 'miragejs/server';
import { CONFIG_KEY, HOOK_KEY } from '@/mirage/config';

export const percySnapshot = (page: Page, testInfo: TestInfo, option?: Parameters<typeof percySnapshot_>[2]) => {
  // Snapshot with a title that mimics @percy/ember
  const titlePath = testInfo.titlePath.length > 2 ? testInfo.titlePath.slice(1) : testInfo.titlePath;
  return percySnapshot_(page, titlePath.join(' | '), option);
};

// By default there's a 400ms delay during development, and 0 delay in testing.
// See: https://miragejs.com/api/classes/server/#timing
const DEFAULT_MIRAGE_CONFIG = { environment: 'test' };

type MirageServer = Server<Registry<typeof MODELS, typeof FACTORIES>>;
type prepareDataFn = (server: MirageServer) => void;

export async function prepareMirage(
  page: Page,
  hook: prepareDataFn,
  options: ServerConfig<typeof MODELS, typeof FACTORIES> = DEFAULT_MIRAGE_CONFIG,
) {
  await page.addInitScript(
    ({ key, options }) => {
      window[`${key}`] = options;
    },
    { key: CONFIG_KEY, options },
  );
  await page.addInitScript(`window['${HOOK_KEY}'] = ${hook.toString()};`);
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

const MODELS = {
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

const FACTORIES = {
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
