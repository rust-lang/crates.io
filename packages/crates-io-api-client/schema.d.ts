/**
 * This file is auto-generated. Do not edit manually.
 *
 * Run `pnpm --filter @crates-io/api-client regenerate` to update this file.
 */

export interface paths {
    "/api/private/crate_owner_invitations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all crate owner invitations for a crate or user. */
        get: operations["list_crate_owner_invitations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/private/session": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** End the current session. */
        delete: operations["end_session"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/private/session/authorize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Complete authentication flow.
         * @description This route is called from the GitHub API OAuth flow after the user accepted or rejected
         *     the data access permissions. It will check the `state` parameter and then call the GitHub API
         *     to exchange the temporary `code` for an API token. The API token is returned together with
         *     the corresponding user information.
         *
         *     see <https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site>
         *
         *     ## Query Parameters
         *
         *     - `code` – temporary code received from the GitHub API  **(Required)**
         *     - `state` – state parameter received from the GitHub API  **(Required)**
         */
        get: operations["authorize_session"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/private/session/begin": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Begin authentication flow.
         * @description This route will return an authorization URL for the GitHub OAuth flow including the crates.io
         *     `client_id` and a randomly generated `state` secret.
         *
         *     see <https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access>
         */
        get: operations["begin_session"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/categories": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all categories. */
        get: operations["list_categories"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/categories/{category}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get category metadata. */
        get: operations["find_category"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/category_slugs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all available category slugs. */
        get: operations["list_category_slugs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/confirm/{email_token}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Marks the email belonging to the given token as verified. */
        put: operations["confirm_user_email"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Returns a list of crates.
         * @description Called in a variety of scenarios in the front end, including:
         *     - Alphabetical listing of crates
         *     - List of crates under a specific owner
         *     - Listing a user's followed crates
         */
        get: operations["list_crates"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/new": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get crate metadata (for the `new` crate).
         * @description This endpoint works around a small limitation in `axum` and is delegating
         *     to the `GET /api/v1/crates/{name}` endpoint internally.
         */
        get: operations["find_new_crate"];
        /**
         * Publish a new crate/version.
         * @description Used by `cargo publish` to publish a new crate or to publish a new version of an
         *     existing crate.
         */
        put: operations["publish"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get crate metadata. */
        get: operations["find_crate"];
        put?: never;
        post?: never;
        /**
         * Delete a crate.
         * @description The crate is immediately deleted from the database, and with a small delay
         *     from the git and sparse index, and the crate file storage.
         *
         *     The crate can only be deleted by the owner of the crate, and only if the
         *     crate has been published for less than 72 hours, or if the crate has a
         *     single owner, has been downloaded less than 1000 times for each month it has
         *     been published, and is not depended upon by any other crate on crates.io.
         */
        delete: operations["delete_crate"];
        options?: never;
        head?: never;
        /** Update crate settings. */
        patch: operations["update_crate"];
        trace?: never;
    };
    "/api/v1/crates/{name}/downloads": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get the download counts for a crate.
         * @description This includes the per-day downloads for the last 90 days and for the
         *     latest 5 versions plus the sum of the rest.
         */
        get: operations["get_crate_downloads"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/follow": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Follow a crate. */
        put: operations["follow_crate"];
        post?: never;
        /** Unfollow a crate. */
        delete: operations["unfollow_crate"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/following": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Check if a crate is followed. */
        get: operations["get_following_crate"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/owner_team": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List team owners of a crate. */
        get: operations["get_team_owners"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/owner_user": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List user owners of a crate. */
        get: operations["get_user_owners"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/owners": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List crate owners. */
        get: operations["list_owners"];
        /** Add crate owners. */
        put: operations["add_owners"];
        post?: never;
        /** Remove crate owners. */
        delete: operations["remove_owners"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/reverse_dependencies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List reverse dependencies of a crate. */
        get: operations["list_reverse_dependencies"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/versions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all versions of a crate. */
        get: operations["list_versions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get crate version metadata. */
        get: operations["find_version"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        /**
         * Update a crate version.
         * @description This endpoint allows updating the `yanked` state of a version, including a yank message.
         */
        patch: operations["update_version"];
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/authors": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get crate version authors.
         * @deprecated
         * @description This endpoint was deprecated by [RFC #3052](https://github.com/rust-lang/rfcs/pull/3052)
         *     and returns an empty list for backwards compatibility reasons.
         */
        get: operations["get_version_authors"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/dependencies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get crate version dependencies.
         * @description This information can also be obtained directly from the index.
         *
         *     In addition to returning cached data from the index, this returns
         *     fields for `id`, `version_id`, and `downloads` (which appears to always
         *     be 0)
         */
        get: operations["get_version_dependencies"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Download a crate version.
         * @description This returns a URL to the location where the crate is stored.
         */
        get: operations["download_version"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/downloads": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get the download counts for a crate version.
         * @description This includes the per-day downloads for the last 90 days.
         */
        get: operations["get_version_downloads"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/readme": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get the readme of a crate version. */
        get: operations["get_version_readme"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/rebuild_docs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Trigger a rebuild for the crate documentation on docs.rs. */
        post: operations["rebuild_version_docs"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/unyank": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Unyank a crate version. */
        put: operations["unyank_version"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/crates/{name}/{version}/yank": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Yank a crate version.
         * @description This does not delete a crate version, it makes the crate
         *     version accessible only to crates that already have a
         *     `Cargo.lock` containing this version.
         *
         *     Notes:
         *
         *     Version deletion is not implemented to avoid breaking builds,
         *     and the goal of yanking a crate is to prevent crates
         *     beginning to depend on the yanked crate version.
         */
        delete: operations["yank_version"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/keywords": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all keywords. */
        get: operations["list_keywords"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/keywords/{keyword}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get keyword metadata. */
        get: operations["find_keyword"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get the currently authenticated user. */
        get: operations["get_authenticated_user"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/crate_owner_invitations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all crate owner invitations for the authenticated user. */
        get: operations["list_crate_owner_invitations_for_user"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/crate_owner_invitations/accept/{token}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Accept a crate owner invitation with a token. */
        put: operations["accept_crate_owner_invitation_with_token"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/crate_owner_invitations/{crate_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Accept or decline a crate owner invitation. */
        put: operations["handle_crate_owner_invitation"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/email_notifications": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Update email notification settings for the authenticated user.
         * @deprecated
         * @description This endpoint was implemented for an experimental feature that was never
         *     fully implemented. It is now deprecated and will be removed in the future.
         */
        put: operations["update_email_notifications"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/tokens": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all API tokens of the authenticated user. */
        get: operations["list_api_tokens"];
        /** Create a new API token. */
        put: operations["create_api_token"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/tokens/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Find API token by id. */
        get: operations["find_api_token"];
        put?: never;
        post?: never;
        /** Revoke API token. */
        delete: operations["revoke_api_token"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/me/updates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List versions of crates that the authenticated user follows. */
        get: operations["get_authenticated_user_updates"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/site_metadata": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get crates.io metadata.
         * @description Returns the current deployed commit SHA1 (or `unknown`), and whether the
         *     system is in read-only mode.
         */
        get: operations["get_site_metadata"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/summary": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get front page data.
         * @description This endpoint returns a summary of the most important data for the front
         *     page of crates.io.
         */
        get: operations["get_summary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/teams/{team}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Find team by login. */
        get: operations["find_team"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tokens/current": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Revoke the current API token.
         * @description This endpoint revokes the API token that is used to authenticate
         *     the request.
         */
        delete: operations["revoke_current_api_token"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/trusted_publishing/github_configs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List Trusted Publishing configurations for GitHub Actions. */
        get: operations["list_trustpub_github_configs"];
        put?: never;
        /** Create a new Trusted Publishing configuration for GitHub Actions. */
        post: operations["create_trustpub_github_config"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/trusted_publishing/github_configs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** Delete Trusted Publishing configuration for GitHub Actions. */
        delete: operations["delete_trustpub_github_config"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/trusted_publishing/gitlab_configs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List Trusted Publishing configurations for GitLab CI/CD. */
        get: operations["list_trustpub_gitlab_configs"];
        put?: never;
        post: operations["create_trustpub_gitlab_config"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/trusted_publishing/gitlab_configs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** Delete Trusted Publishing configuration for GitLab CI/CD. */
        delete: operations["delete_trustpub_gitlab_config"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/trusted_publishing/tokens": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Exchange an OIDC token for a temporary access token. */
        post: operations["exchange_trustpub_token"];
        /**
         * Revoke a temporary access token.
         * @description The access token is expected to be passed in the `Authorization` header
         *     as a `Bearer` token, similar to how it is used in the publish endpoint.
         */
        delete: operations["revoke_trustpub_token"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{id}/resend": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Regenerate and send an email verification token. */
        put: operations["resend_email_verification"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{id}/stats": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get user stats.
         * @description This currently only returns the total number of downloads for crates owned
         *     by the user.
         */
        get: operations["get_user_stats"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{user}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Find user by login. */
        get: operations["find_user"];
        /**
         * Update user settings.
         * @description This endpoint allows users to update their email address and publish notifications settings.
         *
         *     The `id` parameter needs to match the ID of the currently authenticated user.
         */
        put: operations["update_user"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        /** @description The model representing a row in the `api_tokens` database table. */
        ApiToken: {
            /**
             * @description `None` or a list of crate scope patterns (see RFC #2947).
             * @example [
             *       "serde"
             *     ]
             */
            crate_scopes?: string[] | null;
            /**
             * Format: date-time
             * @description The date and time when the token was created.
             * @example 2017-01-06T14:23:11Z
             */
            created_at: string;
            /**
             * @description A list of endpoint scopes or `None` for the `legacy` endpoint scope (see RFC #2947).
             * @example [
             *       "publish-update"
             *     ]
             */
            endpoint_scopes?: components["schemas"]["EndpointScope"][] | null;
            /**
             * Format: date-time
             * @description The date and time when the token will expire, or `null`.
             * @example 2030-10-26T11:32:12Z
             */
            expired_at?: string | null;
            /**
             * Format: int32
             * @description An opaque unique identifier for the token.
             * @example 42
             */
            id: number;
            /**
             * Format: date-time
             * @description The date and time when the token was last used.
             * @example 2021-10-26T11:32:12Z
             */
            last_used_at?: string | null;
            /**
             * @description The name of the token.
             * @example Example API Token
             */
            name: string;
        };
        AuthenticatedUser: {
            /**
             * @description The user's avatar URL, if set.
             * @example https://avatars2.githubusercontent.com/u/1234567?v=4
             */
            avatar?: string | null;
            /**
             * @description The user's email address, if set.
             * @example kate@morgan.dev
             */
            email?: string | null;
            /**
             * @description Whether the user's email address verification email has been sent.
             * @example true
             */
            email_verification_sent: boolean;
            /**
             * @description Whether the user's email address has been verified.
             * @example true
             */
            email_verified: boolean;
            /**
             * Format: int32
             * @description An opaque identifier for the user.
             * @example 42
             */
            id: number;
            /**
             * @description Whether the user is a crates.io administrator.
             * @example false
             */
            is_admin: boolean;
            /**
             * @description The user's login name.
             * @example ghost
             */
            login: string;
            /**
             * @description The user's display name, if set.
             * @example Kate Morgan
             */
            name?: string | null;
            /**
             * @description Whether the user has opted in to receive publish notifications via email.
             * @example true
             */
            publish_notifications: boolean;
            /**
             * @description The user's GitHub profile URL.
             * @example https://github.com/ghost
             */
            url?: string | null;
        };
        Category: {
            /**
             * @description The name of the category.
             * @example Game development
             */
            category: string;
            /**
             * Format: int32
             * @description The total number of crates that have this category.
             * @example 42
             */
            crates_cnt: number;
            /**
             * Format: date-time
             * @description The date and time this category was created.
             * @example 2019-12-13T13:46:41Z
             */
            created_at: string;
            /**
             * @description A description of the category.
             * @example Libraries for creating games.
             */
            description: string;
            /**
             * @description An opaque identifier for the category.
             * @example game-development
             */
            id: string;
            /**
             * @description The parent categories of this category.
             *
             *     This field is only present when the category details are queried,
             *     but not when listing categories.
             * @example []
             */
            parent_categories?: components["schemas"]["Category"][] | null;
            /**
             * @description The "slug" of the category.
             *
             *     See <https://crates.io/category_slugs>.
             * @example game-development
             */
            slug: string;
            /**
             * @description The subcategories of this category.
             *
             *     This field is only present when the category details are queried,
             *     but not when listing categories.
             * @example []
             */
            subcategories?: components["schemas"]["Category"][] | null;
        };
        Crate: {
            /**
             * @deprecated
             * @example []
             */
            badges: Record<string, never>[];
            /**
             * @description The list of categories belonging to this crate.
             * @example null
             */
            categories?: string[] | null;
            /**
             * Format: date-time
             * @description The date and time this crate was created.
             * @example 2019-12-13T13:46:41Z
             */
            created_at: string;
            /**
             * @description The "default" version of this crate.
             *
             *     This version will be displayed by default on the crate's page.
             * @example 1.3.0
             */
            default_version?: string | null;
            /**
             * @description Description of the crate.
             * @example A generic serialization/deserialization framework
             */
            description?: string | null;
            /**
             * @description The URL to the crate's documentation, if set.
             * @example https://docs.rs/serde
             */
            documentation?: string | null;
            /**
             * Format: int64
             * @description The total number of downloads for this crate.
             * @example 123456789
             */
            downloads: number;
            /**
             * @deprecated
             * @description Whether the crate name was an exact match.
             */
            exact_match: boolean;
            /**
             * @description The URL to the crate's homepage, if set.
             * @example https://serde.rs
             */
            homepage?: string | null;
            /**
             * @description An opaque identifier for the crate.
             * @example serde
             */
            id: string;
            /**
             * @description The list of keywords belonging to this crate.
             * @example null
             */
            keywords?: string[] | null;
            /** @description Links to other API endpoints related to this crate. */
            links: components["schemas"]["CrateLinks"];
            /**
             * @deprecated
             * @description The highest version number for this crate that is not a pre-release.
             * @example 1.3.0
             */
            max_stable_version?: string | null;
            /**
             * @deprecated
             * @description The highest version number for this crate.
             * @example 2.0.0-beta.1
             */
            max_version: string;
            /**
             * @description The name of the crate.
             * @example serde
             */
            name: string;
            /**
             * @deprecated
             * @description The most recently published version for this crate.
             * @example 1.2.3
             */
            newest_version: string;
            /**
             * Format: int32
             * @description The total number of versions for this crate.
             * @example 13
             */
            num_versions: number;
            /**
             * Format: int64
             * @description The total number of downloads for this crate in the last 90 days.
             * @example 456789
             */
            recent_downloads?: number | null;
            /**
             * @description The URL to the crate's repository, if set.
             * @example https://github.com/serde-rs/serde
             */
            repository?: string | null;
            /** @description Whether this crate can only be published via Trusted Publishing. */
            trustpub_only: boolean;
            /**
             * Format: date-time
             * @description The date and time this crate was last updated.
             * @example 2019-12-13T13:46:41Z
             */
            updated_at: string;
            /**
             * @description The list of version IDs belonging to this crate.
             * @example null
             */
            versions?: number[] | null;
            /** @description Whether all versions of this crate have been yanked. */
            yanked: boolean;
        };
        CrateLinks: {
            /**
             * @description The API path to this crate's team owners.
             * @example /api/v1/crates/serde/owner_team
             */
            owner_team?: string | null;
            /**
             * @description The API path to this crate's user owners.
             * @example /api/v1/crates/serde/owner_user
             */
            owner_user?: string | null;
            /**
             * @description The API path to this crate's owners.
             * @example /api/v1/crates/serde/owners
             */
            owners?: string | null;
            /**
             * @description The API path to this crate's reverse dependencies.
             * @example /api/v1/crates/serde/reverse_dependencies
             */
            reverse_dependencies: string;
            /**
             * @description The API path to this crate's download statistics.
             * @example /api/v1/crates/serde/downloads
             */
            version_downloads: string;
            /**
             * @description The API path to this crate's versions.
             * @example /api/v1/crates/serde/versions
             */
            versions?: string | null;
        };
        CrateOwnerInvitation: {
            /**
             * Format: int32
             * @description The ID of the crate that the user was invited to be an owner of.
             * @example 123
             */
            crate_id: number;
            /**
             * @description The name of the crate that the user was invited to be an owner of.
             * @example serde
             */
            crate_name: string;
            /**
             * Format: date-time
             * @description The date and time this invitation was created.
             * @example 2019-12-13T13:46:41Z
             */
            created_at: string;
            /**
             * Format: date-time
             * @description The date and time this invitation will expire.
             * @example 2020-01-13T13:46:41Z
             */
            expires_at: string;
            /**
             * Format: int32
             * @description The ID of the user who was invited to be a crate owner.
             * @example 42
             */
            invitee_id: number;
            /**
             * Format: int32
             * @description The ID of the user who sent the invitation.
             * @example 3
             */
            inviter_id: number;
        };
        EncodableApiTokenWithToken: components["schemas"]["ApiToken"] & {
            /**
             * @description The plaintext API token.
             *
             *     Only available when the token is created.
             * @example a1b2c3d4e5f6g7h8i9j0
             */
            token: string;
        };
        EncodableDependency: {
            /**
             * @description The name of the crate this dependency points to.
             * @example serde
             */
            crate_id: string;
            /**
             * @description Whether default features are enabled for this dependency.
             * @example true
             */
            default_features: boolean;
            /**
             * Format: int64
             * @description The total number of downloads for the crate this dependency points to.
             * @example 123456
             */
            downloads: number;
            /** @description The features explicitly enabled for this dependency. */
            features: string[];
            /**
             * Format: int32
             * @description An opaque identifier for the dependency.
             * @example 169
             */
            id: number;
            /**
             * @description The type of dependency this is (normal, dev, or build).
             * @example normal
             */
            kind: string;
            /** @description Whether this dependency is optional. */
            optional: boolean;
            /**
             * @description The version requirement for this dependency.
             * @example ^1
             */
            req: string;
            /** @description The target platform for this dependency, if any. */
            target?: string | null;
            /**
             * Format: int32
             * @description The ID of the version this dependency belongs to.
             * @example 42
             */
            version_id: number;
        };
        /** @enum {string} */
        EndpointScope: "publish-new" | "publish-update" | "trusted-publishing" | "yank" | "change-owners";
        GitHubConfig: {
            /** @example regex */
            crate: string;
            /** Format: date-time */
            created_at: string;
            /** @example null */
            environment?: string | null;
            /**
             * Format: int32
             * @example 42
             */
            id: number;
            /** @example regex */
            repository_name: string;
            /** @example rust-lang */
            repository_owner: string;
            /**
             * Format: int32
             * @example 5430905
             */
            repository_owner_id: number;
            /** @example ci.yml */
            workflow_filename: string;
        };
        GitLabConfig: {
            /** @example regex */
            crate: string;
            /** Format: date-time */
            created_at: string;
            /** @example null */
            environment?: string | null;
            /**
             * Format: int32
             * @example 42
             */
            id: number;
            /** @example rust-lang */
            namespace: string;
            /** @example null */
            namespace_id?: string | null;
            /** @example regex */
            project: string;
            /** @example .gitlab-ci.yml */
            workflow_filepath: string;
        };
        Keyword: {
            /**
             * Format: int32
             * @description The total number of crates that have this keyword.
             * @example 42
             */
            crates_cnt: number;
            /**
             * Format: date-time
             * @description The date and time this keyword was created.
             * @example 2017-01-06T14:23:11Z
             */
            created_at: string;
            /**
             * @description An opaque identifier for the keyword.
             * @example http
             */
            id: string;
            /**
             * @description The keyword itself.
             * @example http
             */
            keyword: string;
        };
        LegacyCrateOwnerInvitation: {
            /**
             * Format: int32
             * @description The ID of the crate that the user was invited to be an owner of.
             * @example 123
             */
            crate_id: number;
            /**
             * @description The name of the crate that the user was invited to be an owner of.
             * @example serde
             */
            crate_name: string;
            /**
             * Format: date-time
             * @description The date and time this invitation was created.
             * @example 2019-12-13T13:46:41Z
             */
            created_at: string;
            /**
             * Format: date-time
             * @description The date and time this invitation will expire.
             * @example 2020-01-13T13:46:41Z
             */
            expires_at: string;
            /**
             * @description The username of the user who sent the invitation.
             * @example ghost
             */
            invited_by_username: string;
            /**
             * Format: int32
             * @description The ID of the user who was invited to be a crate owner.
             * @example 42
             */
            invitee_id: number;
            /**
             * Format: int32
             * @description The ID of the user who sent the invitation.
             * @example 3
             */
            inviter_id: number;
        };
        NewGitHubConfig: {
            /** @example regex */
            crate: string;
            /** @example null */
            environment?: string | null;
            /** @example regex */
            repository_name: string;
            /** @example rust-lang */
            repository_owner: string;
            /** @example ci.yml */
            workflow_filename: string;
        };
        NewGitLabConfig: {
            /** @example regex */
            crate: string;
            /** @example null */
            environment?: string | null;
            /** @example rust-lang */
            namespace: string;
            /** @example regex */
            project: string;
            /** @example .gitlab-ci.yml */
            workflow_filepath: string;
        };
        Owner: {
            /**
             * @description The avatar URL of the team or user.
             * @example https://avatars2.githubusercontent.com/u/1234567?v=4
             */
            avatar?: string | null;
            /**
             * Format: int32
             * @description The opaque identifier for the team or user, depending on the `kind` field.
             * @example 42
             */
            id: number;
            /**
             * @description The kind of the owner (`user` or `team`).
             * @example user
             */
            kind: string;
            /**
             * @description The login name of the team or user.
             * @example ghost
             */
            login: string;
            /**
             * @description The display name of the team or user.
             * @example Kate Morgan
             */
            name?: string | null;
            /**
             * @description The URL to the owner's profile.
             * @example https://github.com/ghost
             */
            url?: string | null;
        };
        PatchRequest: {
            /** @description The crate settings to update. */
            crate: components["schemas"]["PatchRequestCrate"];
        };
        PatchRequestCrate: {
            /** @description Whether this crate can only be published via Trusted Publishing. */
            trustpub_only?: boolean | null;
        };
        PublishWarnings: {
            /**
             * @deprecated
             * @example []
             */
            invalid_badges: string[];
            /** @example [] */
            invalid_categories: string[];
            /** @example [] */
            other: string[];
        };
        Slug: {
            /**
             * @description A description of the category.
             * @example Libraries for creating games.
             */
            description: string;
            /**
             * @description An opaque identifier for the category.
             * @example game-development
             */
            id: string;
            /**
             * @description The "slug" of the category.
             *
             *     See <https://crates.io/category_slugs>.
             * @example game-development
             */
            slug: string;
        };
        Team: {
            /**
             * @description The avatar URL of the team.
             * @example https://avatars2.githubusercontent.com/u/1234567?v=4
             */
            avatar?: string | null;
            /**
             * Format: int32
             * @description An opaque identifier for the team.
             * @example 42
             */
            id: number;
            /**
             * @description The login name of the team.
             * @example github:rust-lang:crates-io
             */
            login: string;
            /**
             * @description The display name of the team.
             * @example Crates.io team
             */
            name?: string | null;
            /**
             * @description The GitHub profile URL of the team.
             * @example https://github.com/rust-lang
             */
            url?: string | null;
        };
        User: {
            /**
             * @description The user's avatar URL, if set.
             * @example https://avatars2.githubusercontent.com/u/1234567?v=4
             */
            avatar?: string | null;
            /**
             * Format: int32
             * @description An opaque identifier for the user.
             * @example 42
             */
            id: number;
            /**
             * @description The user's login name.
             * @example ghost
             */
            login: string;
            /**
             * @description The user's display name, if set.
             * @example Kate Morgan
             */
            name?: string | null;
            /**
             * @description The user's GitHub profile URL.
             * @example https://github.com/ghost
             */
            url: string;
        };
        Version: {
            /** @description A list of actions performed on this version. */
            audit_actions: {
                /**
                 * @description The action that was performed.
                 * @example publish
                 */
                action: string;
                /**
                 * Format: date-time
                 * @description The date and time the action was performed.
                 * @example 2019-12-13T13:46:41Z
                 */
                time: string;
                /** @description The user who performed the action. */
                user: components["schemas"]["User"];
            }[];
            /**
             * @description The names of the binaries provided by this version, if any.
             * @example []
             */
            bin_names?: (string | null)[] | null;
            /**
             * @description The SHA256 checksum of the compressed crate file encoded as a
             *     hexadecimal string.
             * @example e8dfc9d19bdbf6d17e22319da49161d5d0108e4188e8b680aef6299eed22df60
             */
            checksum: string;
            /**
             * @description The name of the crate.
             * @example serde
             */
            crate: string;
            /**
             * Format: int32
             * @description The size of the compressed crate file in bytes.
             * @example 1234
             */
            crate_size: number;
            /**
             * Format: date-time
             * @description The date and time this version was created.
             * @example 2019-12-13T13:46:41Z
             */
            created_at: string;
            /**
             * @description The description of this version of the crate.
             * @example A generic serialization/deserialization framework
             */
            description?: string | null;
            /**
             * @description The API path to download the crate.
             * @example /api/v1/crates/serde/1.0.0/download
             */
            dl_path: string;
            /**
             * @description The URL to the crate's documentation, if set.
             * @example https://docs.rs/serde
             */
            documentation?: string | null;
            /**
             * Format: int32
             * @description The total number of downloads for this version.
             * @example 123456
             */
            downloads: number;
            /**
             * @description The Rust Edition used to compile this version, if set.
             * @example 2021
             */
            edition?: string | null;
            /** @description The features defined by this version. */
            features: Record<string, never>;
            /**
             * @description Whether this version can be used as a library.
             * @example true
             */
            has_lib?: boolean | null;
            /**
             * @description The URL to the crate's homepage, if set.
             * @example https://serde.rs
             */
            homepage?: string | null;
            /**
             * Format: int32
             * @description An opaque identifier for the version.
             * @example 42
             */
            id: number;
            /**
             * @description The name of the native library this version links with, if any.
             * @example git2
             */
            lib_links?: string | null;
            /**
             * @description The license of this version of the crate.
             * @example MIT
             */
            license?: string | null;
            /**
             * @description Line count statistics for this version.
             *
             *     Status: **Unstable**
             *
             *     This field may be `null` until the version has been analyzed, which
             *     happens in an asynchronous background job.
             */
            linecounts: Record<string, never>;
            /** @description Links to other API endpoints related to this version. */
            links: components["schemas"]["VersionLinks"];
            /**
             * @description The version number.
             * @example 1.0.0
             */
            num: string;
            published_by?: null | components["schemas"]["User"];
            /**
             * @description The API path to download the crate's README file as HTML code.
             * @example /api/v1/crates/serde/1.0.0/readme
             */
            readme_path: string;
            /**
             * @description The URL to the crate's repository, if set.
             * @example https://github.com/serde-rs/serde
             */
            repository?: string | null;
            /**
             * @description The minimum version of the Rust compiler required to compile
             *     this version, if set.
             * @example 1.31
             */
            rust_version?: string | null;
            /**
             * @description Information about the trusted publisher that published this version, if any.
             *
             *     Status: **Unstable**
             *
             *     This field is filled if the version was published via trusted publishing
             *     (e.g., GitHub Actions) rather than a regular API token.
             *
             *     The exact structure of this field depends on the `provider` field
             *     inside it.
             */
            trustpub_data?: Record<string, never> | null;
            /**
             * Format: date-time
             * @description The date and time this version was last updated (i.e. yanked or unyanked).
             * @example 2019-12-13T13:46:41Z
             */
            updated_at: string;
            /**
             * @description The message given when this version was yanked, if any.
             * @example Security vulnerability
             */
            yank_message?: string | null;
            /**
             * @description Whether this version has been yanked.
             * @example false
             */
            yanked: boolean;
        };
        VersionDownload: {
            /**
             * @description The date this download count is for.
             * @example 2019-12-13
             */
            date: string;
            /**
             * Format: int32
             * @description The number of downloads for this version on the given date.
             * @example 123
             */
            downloads: number;
            /**
             * Format: int32
             * @description The ID of the version this download count is for.
             * @example 42
             */
            version: number;
        };
        VersionLinks: {
            /**
             * @deprecated
             * @description The API path to download this version's authors.
             * @example /api/v1/crates/serde/1.0.0/authors
             */
            authors: string;
            /**
             * @description The API path to download this version's dependencies.
             * @example /api/v1/crates/serde/1.0.0/dependencies
             */
            dependencies: string;
            /**
             * @description The API path to download this version's download numbers.
             * @example /api/v1/crates/serde/1.0.0/downloads
             */
            version_downloads: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    list_crate_owner_invitations: {
        parameters: {
            query?: {
                /**
                 * @description Filter crate owner invitations by crate name.
                 *
                 *     Only crate owners can query pending invitations for their crate.
                 */
                crate_name?: string;
                /**
                 * @description The ID of the user who was invited to be a crate owner.
                 *
                 *     This parameter needs to match the authenticated user's ID.
                 */
                invitee_id?: number;
                /**
                 * @description The page number to request.
                 *
                 *     This parameter is mutually exclusive with `seek` and not supported for
                 *     all requests.
                 */
                page?: number;
                /** @description The number of items to request per page. */
                per_page?: number;
                /**
                 * @description The seek key to request.
                 *
                 *     This parameter is mutually exclusive with `page` and not supported for
                 *     all requests.
                 *
                 *     The seek key can usually be found in the `meta.next_page` field of
                 *     paginated responses.
                 */
                seek?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The list of crate owner invitations. */
                        invitations: components["schemas"]["CrateOwnerInvitation"][];
                        meta: {
                            /**
                             * @description Query parameter string to fetch the next page of results.
                             * @example ?seek=c0ffee
                             */
                            next_page?: string | null;
                        };
                        /** @description The list of users referenced in the crate owner invitations. */
                        users: components["schemas"]["User"][];
                    };
                };
            };
        };
    };
    end_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    authorize_session: {
        parameters: {
            query: {
                /**
                 * @description Temporary code received from the GitHub API.
                 * @example 901dd10e07c7e9fa1cd5
                 */
                code: string;
                /**
                 * @description State parameter received from the GitHub API.
                 * @example fYcUY3FMdUUz00FC7vLT7A
                 */
                state: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The crates that the authenticated user owns. */
                        owned_crates: {
                            /** @deprecated */
                            email_notifications: boolean;
                            /**
                             * Format: int32
                             * @description The opaque identifier of the crate.
                             * @example 123
                             */
                            id: number;
                            /**
                             * @description The name of the crate.
                             * @example serde
                             */
                            name: string;
                        }[];
                        /** @description The authenticated user. */
                        user: components["schemas"]["AuthenticatedUser"];
                    };
                };
            };
        };
    };
    begin_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example b84a63c4ea3fcb4ac84 */
                        state: string;
                        /** @example https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg */
                        url: string;
                    };
                };
            };
        };
    };
    list_categories: {
        parameters: {
            query?: {
                /**
                 * @description The sort order of the categories.
                 *
                 *     Valid values: `alpha`, and `crates`.
                 *
                 *     Defaults to `alpha`.
                 */
                sort?: string;
                /**
                 * @description The page number to request.
                 *
                 *     This parameter is mutually exclusive with `seek` and not supported for
                 *     all requests.
                 */
                page?: number;
                /** @description The number of items to request per page. */
                per_page?: number;
                /**
                 * @description The seek key to request.
                 *
                 *     This parameter is mutually exclusive with `page` and not supported for
                 *     all requests.
                 *
                 *     The seek key can usually be found in the `meta.next_page` field of
                 *     paginated responses.
                 */
                seek?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The list of categories. */
                        categories: components["schemas"]["Category"][];
                        meta: {
                            /**
                             * Format: int64
                             * @description The total number of categories.
                             * @example 123
                             */
                            total: number;
                        };
                    };
                };
            };
        };
    };
    find_category: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the category */
                category: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        category: components["schemas"]["Category"];
                    };
                };
            };
        };
    };
    list_category_slugs: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The list of category slugs. */
                        category_slugs: components["schemas"]["Slug"][];
                    };
                };
            };
        };
    };
    confirm_user_email: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Secret verification token sent to the user's email address */
                email_token: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    list_crates: {
        parameters: {
            query?: {
                /**
                 * @description The sort order of the crates.
                 *
                 *     Valid values: `alphabetical`, `relevance`, `downloads`,
                 *     `recent-downloads`, `recent-updates`, `new`.
                 *
                 *     Defaults to `relevance` if `q` is set, otherwise `alphabetical`.
                 */
                sort?: string;
                /** @description A search query string. */
                q?: string;
                /**
                 * @description Set to `yes` to include yanked crates.
                 * @example yes
                 */
                include_yanked?: string;
                /**
                 * @description If set, only return crates that belong to this category, or one
                 *     of its subcategories.
                 */
                category?: string;
                /**
                 * @description If set, only return crates matching all the given keywords.
                 *
                 *     This parameter expects a space-separated list of keywords.
                 */
                all_keywords?: string;
                /**
                 * @description If set, only return crates matching the given keyword
                 *     (ignored if `all_keywords` is set).
                 */
                keyword?: string;
                /**
                 * @description If set, only return crates with names that start with the given letter
                 *     (ignored if `all_keywords` or `keyword` are set).
                 */
                letter?: string;
                /**
                 * @description If set, only crates owned by the given crates.io user ID are returned
                 *     (ignored if `all_keywords`, `keyword`, or `letter` are set).
                 */
                user_id?: number;
                /**
                 * @description If set, only crates owned by the given crates.io team ID are returned
                 *     (ignored if `all_keywords`, `keyword`, `letter`, or `user_id` are set).
                 */
                team_id?: number;
                /**
                 * @description If set, only crates owned by users the current user follows are returned
                 *     (ignored if `all_keywords`, `keyword`, `letter`, `user_id`,
                 *     or `team_id` are set).
                 *
                 *     The exact value of this parameter is ignored, but it must not be empty.
                 * @example yes
                 */
                following?: string;
                /**
                 * @description If set, only crates with the specified names are returned (ignored
                 *     if `all_keywords`, `keyword`, `letter`, `user_id`, `team_id`,
                 *     or `following` are set).
                 */
                "ids[]"?: string[];
                /**
                 * @description The page number to request.
                 *
                 *     This parameter is mutually exclusive with `seek` and not supported for
                 *     all requests.
                 */
                page?: number;
                /** @description The number of items to request per page. */
                per_page?: number;
                /**
                 * @description The seek key to request.
                 *
                 *     This parameter is mutually exclusive with `page` and not supported for
                 *     all requests.
                 *
                 *     The seek key can usually be found in the `meta.next_page` field of
                 *     paginated responses.
                 */
                seek?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        crates: components["schemas"]["Crate"][];
                        meta: {
                            /**
                             * @description Query string to the next page of results, if any.
                             * @example ?page=3
                             */
                            next_page?: string | null;
                            /**
                             * @description Query string to the previous page of results, if any.
                             * @example ?page=1
                             */
                            prev_page?: string | null;
                            /**
                             * Format: int64
                             * @description The total number of crates that match the query.
                             * @example 123
                             */
                            total: number;
                        };
                    };
                };
            };
        };
    };
    find_new_crate: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /**
                         * @description The categories of the crate.
                         * @example null
                         */
                        categories?: components["schemas"]["Category"][] | null;
                        /** @description The crate metadata. */
                        crate: components["schemas"]["Crate"];
                        /**
                         * @description The keywords of the crate.
                         * @example null
                         */
                        keywords?: components["schemas"]["Keyword"][] | null;
                        /**
                         * @description The versions of the crate.
                         * @example null
                         */
                        versions?: components["schemas"]["Version"][] | null;
                    };
                };
            };
        };
    };
    publish: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        crate: components["schemas"]["Crate"];
                        warnings: components["schemas"]["PublishWarnings"];
                    };
                };
            };
        };
    };
    find_crate: {
        parameters: {
            query?: {
                /**
                 * @description Additional data to include in the response.
                 *
                 *     Valid values: `versions`, `keywords`, `categories`, `badges`,
                 *     `downloads`, `default_version`, or `full`.
                 *
                 *     Defaults to `full` for backwards compatibility.
                 *
                 *     **Note**: `versions` and `default_version` share the same key `versions`, therefore `default_version` will be ignored if both are provided.
                 *
                 *     This parameter expects a comma-separated list of values.
                 */
                include?: string;
            };
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /**
                         * @description The categories of the crate.
                         * @example null
                         */
                        categories?: components["schemas"]["Category"][] | null;
                        /** @description The crate metadata. */
                        crate: components["schemas"]["Crate"];
                        /**
                         * @description The keywords of the crate.
                         * @example null
                         */
                        keywords?: components["schemas"]["Keyword"][] | null;
                        /**
                         * @description The versions of the crate.
                         * @example null
                         */
                        versions?: components["schemas"]["Version"][] | null;
                    };
                };
            };
        };
    };
    delete_crate: {
        parameters: {
            query?: {
                message?: string;
            };
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_crate: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PatchRequest"];
            };
        };
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The updated crate metadata. */
                        crate: components["schemas"]["Crate"];
                    };
                };
            };
        };
    };
    get_crate_downloads: {
        parameters: {
            query?: {
                /**
                 * @description Additional data to include in the response.
                 *
                 *     Valid values: `versions`.
                 *
                 *     Defaults to no additional data.
                 *
                 *     This parameter expects a comma-separated list of values.
                 */
                include?: string;
            };
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        meta: {
                            extra_downloads: {
                                /**
                                 * @description The date this download count is for.
                                 * @example 2019-12-13
                                 */
                                date: string;
                                /**
                                 * Format: int64
                                 * @description The number of downloads on the given date.
                                 * @example 123
                                 */
                                downloads: number;
                            }[];
                        };
                        /** @description The per-day download counts for the last 90 days. */
                        version_downloads: components["schemas"]["VersionDownload"][];
                        /**
                         * @description The versions referenced in the download counts, if `?include=versions`
                         *     was requested.
                         */
                        versions?: components["schemas"]["Version"][] | null;
                    };
                };
            };
        };
    };
    follow_crate: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    unfollow_crate: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    get_following_crate: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description Whether the authenticated user is following the crate. */
                        following: boolean;
                    };
                };
            };
        };
    };
    get_team_owners: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        teams: components["schemas"]["Owner"][];
                    };
                };
            };
        };
    };
    get_user_owners: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        users: components["schemas"]["Owner"][];
                    };
                };
            };
        };
    };
    list_owners: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        users: components["schemas"]["Owner"][];
                    };
                };
            };
        };
    };
    add_owners: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    /**
                     * @description List of owner login names to add or remove.
                     *
                     *     For users, use just the username (e.g., `"octocat"`).
                     *     For GitHub teams, use the format `github:org:team` (e.g., `"github:rust-lang:owners"`).
                     * @example [
                     *       "octocat",
                     *       "github:rust-lang:owners"
                     *     ]
                     */
                    owners: string[];
                };
            };
        };
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /**
                         * @description A message describing the result of the operation.
                         * @example user ghost has been invited to be an owner of crate serde
                         */
                        msg: string;
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    remove_owners: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    /**
                     * @description List of owner login names to add or remove.
                     *
                     *     For users, use just the username (e.g., `"octocat"`).
                     *     For GitHub teams, use the format `github:org:team` (e.g., `"github:rust-lang:owners"`).
                     * @example [
                     *       "octocat",
                     *       "github:rust-lang:owners"
                     *     ]
                     */
                    owners: string[];
                };
            };
        };
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /**
                         * @description A message describing the result of the operation.
                         * @example user ghost has been invited to be an owner of crate serde
                         */
                        msg: string;
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    list_reverse_dependencies: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The list of reverse dependencies of the crate. */
                        dependencies: components["schemas"]["EncodableDependency"][];
                        meta: {
                            /**
                             * Format: int64
                             * @example 32
                             */
                            total: number;
                        };
                        /** @description The versions referenced in the `dependencies` field. */
                        versions: components["schemas"]["Version"][];
                    };
                };
            };
        };
    };
    list_versions: {
        parameters: {
            query?: {
                /**
                 * @description Additional data to include in the response.
                 *
                 *     Valid values: `release_tracks`.
                 *
                 *     Defaults to no additional data.
                 *
                 *     This parameter expects a comma-separated list of values.
                 */
                include?: string;
                /**
                 * @description The sort order of the versions.
                 *
                 *     Valid values: `date`, and `semver`.
                 *
                 *     Defaults to `semver`.
                 */
                sort?: string;
                /** @description If set, only versions with the specified semver strings are returned. */
                "nums[]"?: string[];
                /**
                 * @description The page number to request.
                 *
                 *     This parameter is mutually exclusive with `seek` and not supported for
                 *     all requests.
                 */
                page?: number;
                /** @description The number of items to request per page. */
                per_page?: number;
                /**
                 * @description The seek key to request.
                 *
                 *     This parameter is mutually exclusive with `page` and not supported for
                 *     all requests.
                 *
                 *     The seek key can usually be found in the `meta.next_page` field of
                 *     paginated responses.
                 */
                seek?: string;
            };
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        meta: {
                            /**
                             * @description Query string to the next page of results, if any.
                             * @example ?page=3
                             */
                            next_page?: string | null;
                            /**
                             * @description Additional data about the crate's release tracks,
                             *     if `?include=release_tracks` is used.
                             */
                            release_tracks?: Record<string, never> | null;
                            /**
                             * Format: int64
                             * @description The total number of versions belonging to the crate.
                             * @example 123
                             */
                            total: number;
                        };
                        versions: components["schemas"]["Version"][];
                    };
                };
            };
        };
    };
    find_version: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        version: components["schemas"]["Version"];
                    };
                };
            };
        };
    };
    update_version: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        version: components["schemas"]["Version"];
                    };
                };
            };
        };
    };
    get_version_authors: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_version_dependencies: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        dependencies: components["schemas"]["EncodableDependency"][];
                    };
                };
            };
        };
    };
    download_version: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response (for `content-type: application/json`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /**
                         * @description The URL to the crate file.
                         * @example https://static.crates.io/crates/serde/serde-1.0.0.crate
                         */
                        url: string;
                    };
                };
            };
            /** @description Successful Response (default) */
            302: {
                headers: {
                    /** @description The URL to the crate file. */
                    location?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_version_downloads: {
        parameters: {
            query?: {
                /**
                 * @description Only return download counts before this date.
                 * @example 2024-06-28
                 */
                before_date?: string;
            };
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        version_downloads: components["schemas"]["VersionDownload"][];
                    };
                };
            };
        };
    };
    get_version_readme: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response (for `content-type: application/json`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /**
                         * @description The URL to the readme file.
                         * @example https://static.crates.io/readmes/serde/serde-1.0.0.html
                         */
                        url: string;
                    };
                };
            };
            /** @description Successful Response (default) */
            302: {
                headers: {
                    /** @description The URL to the readme file. */
                    location?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    rebuild_version_docs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    unyank_version: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    yank_version: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Name of the crate */
                name: string;
                /**
                 * @description Version number
                 * @example 1.0.0
                 */
                version: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    list_keywords: {
        parameters: {
            query?: {
                /**
                 * @description The sort order of the keywords.
                 *
                 *     Valid values: `alpha`, and `crates`.
                 *
                 *     Defaults to `alpha`.
                 */
                sort?: string;
                /**
                 * @description The page number to request.
                 *
                 *     This parameter is mutually exclusive with `seek` and not supported for
                 *     all requests.
                 */
                page?: number;
                /** @description The number of items to request per page. */
                per_page?: number;
                /**
                 * @description The seek key to request.
                 *
                 *     This parameter is mutually exclusive with `page` and not supported for
                 *     all requests.
                 *
                 *     The seek key can usually be found in the `meta.next_page` field of
                 *     paginated responses.
                 */
                seek?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The list of keywords. */
                        keywords: components["schemas"]["Keyword"][];
                        meta: {
                            /**
                             * Format: int64
                             * @description The total number of keywords.
                             * @example 123
                             */
                            total: number;
                        };
                    };
                };
            };
        };
    };
    find_keyword: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description The keyword to find */
                keyword: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        keyword: components["schemas"]["Keyword"];
                    };
                };
            };
        };
    };
    get_authenticated_user: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The crates that the authenticated user owns. */
                        owned_crates: {
                            /** @deprecated */
                            email_notifications: boolean;
                            /**
                             * Format: int32
                             * @description The opaque identifier of the crate.
                             * @example 123
                             */
                            id: number;
                            /**
                             * @description The name of the crate.
                             * @example serde
                             */
                            name: string;
                        }[];
                        /** @description The authenticated user. */
                        user: components["schemas"]["AuthenticatedUser"];
                    };
                };
            };
        };
    };
    list_crate_owner_invitations_for_user: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The list of crate owner invitations. */
                        crate_owner_invitations: components["schemas"]["LegacyCrateOwnerInvitation"][];
                        /** @description The list of users referenced in the crate owner invitations. */
                        users: components["schemas"]["User"][];
                    };
                };
            };
        };
    };
    accept_crate_owner_invitation_with_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Secret token sent to the user's email address */
                token: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        crate_owner_invitation: {
                            /**
                             * @description Whether the invitation was accepted.
                             * @example true
                             */
                            accepted: boolean;
                            /**
                             * Format: int32
                             * @description The opaque identifier for the crate this invitation is for.
                             * @example 42
                             */
                            crate_id: number;
                        };
                    };
                };
            };
        };
    };
    handle_crate_owner_invitation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the crate */
                crate_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        crate_owner_invitation: {
                            /**
                             * @description Whether the invitation was accepted.
                             * @example true
                             */
                            accepted: boolean;
                            /**
                             * Format: int32
                             * @description The opaque identifier for the crate this invitation is for.
                             * @example 42
                             */
                            crate_id: number;
                        };
                    };
                };
            };
        };
    };
    update_email_notifications: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    list_api_tokens: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        api_tokens: components["schemas"]["ApiToken"][];
                    };
                };
            };
        };
    };
    create_api_token: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        api_token: components["schemas"]["EncodableApiTokenWithToken"];
                    };
                };
            };
        };
    };
    find_api_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the API token */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        api_token: components["schemas"]["ApiToken"];
                    };
                };
            };
        };
    };
    revoke_api_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the API token */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    get_authenticated_user_updates: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        meta: {
                            /** @description Whether there are more versions to be loaded. */
                            more: boolean;
                        };
                        /** @description The list of recent versions of crates that the authenticated user follows. */
                        versions: components["schemas"]["Version"][];
                    };
                };
            };
        };
    };
    get_site_metadata: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description Optional banner message to display on all pages. */
                        banner_message?: string | null;
                        /**
                         * @description The SHA1 of the currently deployed commit.
                         * @example 0aebe2cdfacae1229b93853b1c58f9352195f081
                         */
                        commit: string;
                        /**
                         * @description The SHA1 of the currently deployed commit.
                         * @example 0aebe2cdfacae1229b93853b1c58f9352195f081
                         */
                        deployed_sha: string;
                        /** @description Whether the crates.io service is in read-only mode. */
                        read_only: boolean;
                    };
                };
            };
        };
    };
    get_summary: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The 10 most recently updated crates. */
                        just_updated: components["schemas"]["Crate"][];
                        /** @description The 10 crates with the highest total number of downloads. */
                        most_downloaded: components["schemas"]["Crate"][];
                        /** @description The 10 crates with the highest number of downloads within the last 90 days. */
                        most_recently_downloaded: components["schemas"]["Crate"][];
                        /** @description The 10 most recently created crates. */
                        new_crates: components["schemas"]["Crate"][];
                        /**
                         * Format: int64
                         * @description The total number of crates on crates.io.
                         * @example 123456
                         */
                        num_crates: number;
                        /**
                         * Format: int64
                         * @description The total number of downloads across all crates.
                         * @example 123456789
                         */
                        num_downloads: number;
                        /** @description The 10 most popular categories. */
                        popular_categories: components["schemas"]["Category"][];
                        /** @description The 10 most popular keywords. */
                        popular_keywords: components["schemas"]["Keyword"][];
                    };
                };
            };
        };
    };
    find_team: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /**
                 * @description Name of the team
                 * @example github:rust-lang:crates-io
                 */
                team: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        team: components["schemas"]["Team"];
                    };
                };
            };
        };
    };
    revoke_current_api_token: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_trustpub_github_configs: {
        parameters: {
            query?: {
                /** @description Name of the crate to list Trusted Publishing configurations for. */
                crate?: string;
                /** @description User ID to list Trusted Publishing configurations for all crates owned by the user. */
                user_id?: number;
                /**
                 * @description The page number to request.
                 *
                 *     This parameter is mutually exclusive with `seek` and not supported for
                 *     all requests.
                 */
                page?: number;
                /** @description The number of items to request per page. */
                per_page?: number;
                /**
                 * @description The seek key to request.
                 *
                 *     This parameter is mutually exclusive with `page` and not supported for
                 *     all requests.
                 *
                 *     The seek key can usually be found in the `meta.next_page` field of
                 *     paginated responses.
                 */
                seek?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        github_configs: components["schemas"]["GitHubConfig"][];
                        meta: {
                            /**
                             * @description Query string to the next page of results, if any.
                             * @example ?seek=abc123
                             */
                            next_page?: string | null;
                            /**
                             * Format: int64
                             * @description The total number of GitHub configs belonging to the crate.
                             * @example 42
                             */
                            total: number;
                        };
                    };
                };
            };
        };
    };
    create_trustpub_github_config: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    github_config: components["schemas"]["NewGitHubConfig"];
                };
            };
        };
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        github_config: components["schemas"]["GitHubConfig"];
                    };
                };
            };
        };
    };
    delete_trustpub_github_config: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the Trusted Publishing configuration */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    list_trustpub_gitlab_configs: {
        parameters: {
            query?: {
                /** @description Name of the crate to list Trusted Publishing configurations for. */
                crate?: string;
                /** @description User ID to list Trusted Publishing configurations for all crates owned by the user. */
                user_id?: number;
                /**
                 * @description The page number to request.
                 *
                 *     This parameter is mutually exclusive with `seek` and not supported for
                 *     all requests.
                 */
                page?: number;
                /** @description The number of items to request per page. */
                per_page?: number;
                /**
                 * @description The seek key to request.
                 *
                 *     This parameter is mutually exclusive with `page` and not supported for
                 *     all requests.
                 *
                 *     The seek key can usually be found in the `meta.next_page` field of
                 *     paginated responses.
                 */
                seek?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        gitlab_configs: components["schemas"]["GitLabConfig"][];
                        meta: {
                            /**
                             * @description Query string to the next page of results, if any.
                             * @example ?seek=abc123
                             */
                            next_page?: string | null;
                            /**
                             * Format: int64
                             * @description The total number of GitLab configs belonging to the crate.
                             * @example 42
                             */
                            total: number;
                        };
                    };
                };
            };
        };
    };
    create_trustpub_gitlab_config: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    gitlab_config: components["schemas"]["NewGitLabConfig"];
                };
            };
        };
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        gitlab_config: components["schemas"]["GitLabConfig"];
                    };
                };
            };
        };
    };
    delete_trustpub_gitlab_config: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the Trusted Publishing configuration */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    exchange_trustpub_token: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": {
                    jwt: string;
                };
            };
        };
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        token: string;
                    };
                };
            };
        };
    };
    revoke_trustpub_token: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    resend_email_verification: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the user */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
    get_user_stats: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the user */
                id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /**
                         * Format: int64
                         * @description The total number of downloads for crates owned by the user.
                         * @example 123456789
                         */
                        total_downloads: number;
                    };
                };
            };
        };
    };
    find_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Login name of the user */
                user: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        user: components["schemas"]["User"];
                    };
                };
            };
        };
    };
    update_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ID of the user */
                user: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Successful Response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @example true */
                        ok: boolean;
                    };
                };
            };
        };
    };
}
