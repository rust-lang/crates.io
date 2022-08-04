//! Endpoints for managing per user webhook settings

use diesel::associations::Identifiable;
use url::Url;

use crate::controllers::frontend_prelude::*;
use crate::db::DieselPooledConn;
use crate::models::{NewWebhook, Webhook};
use crate::schema::*;

use crate::util::read_fill;
use serde_json as json;

fn build_webhook(
    req: &dyn RequestExt,
    conn: &DieselPooledConn<'_>,
    user_id: i32,
) -> AppResult<NewWebhook> {
    let url = Url::parse(&req.params()["webhook_url"])?;

    Ok(NewWebhook {
        owner_id: user_id,
        webhook_url: url.to_string(),
    })
}

pub fn new(req: &mut dyn RequestExt) -> EndpointResult {
    #[derive(Deserialize, Serialize)]
    struct NewWebhookUrl {
        url: Url,
    }

    #[derive(Deserialize, Serialize)]
    struct NewWebhookRequest {
        webhook: NewWebhookUrl,
    }

    let length = req
        .content_length()
        .ok_or_else(|| bad_request("missing header: Content-Length"))?;

    let mut json = vec![0; length as usize];
    read_fill(req.body(), &mut json)?;

    let json =
        String::from_utf8(json).map_err(|_| bad_request(&"json body was not valid utf-8"))?;

    let new: NewWebhookRequest = json::from_str(&json)
        .map_err(|e| bad_request(&format!("invalid new webhook request: {e:?}")))?;

    let url = new.webhook.url;

    let user_id = req.authenticate()?.user_id();

    let values = NewWebhook {
        owner_id: user_id,
        webhook_url: url.to_string(),
    };

    let conn = req.db_write()?;
    let webhook = diesel::insert_into(webhooks::table)
        .values(values)
        .on_conflict_do_nothing()
        .execute(&*conn)?;

    ok_true()
}

/// Handles the `PUT ???` route
pub fn create_webhook(req: &mut dyn RequestExt) -> EndpointResult {
    let user_id = req.authenticate()?.user_id();
    let conn = req.db_write()?;

    let webhook = build_webhook(req, &conn, user_id)?;

    diesel::insert_into(webhooks::table)
        .values(&webhook)
        .on_conflict_do_nothing()
        .execute(&*conn)?;

    ok_true()
}
