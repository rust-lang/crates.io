use crates_io_database::models::{Crate, CrateOwner, Team, User};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;

pub async fn add_team_to_crate(
    t: &Team,
    krate: &Crate,
    u: &User,
    conn: &mut AsyncPgConnection,
) -> QueryResult<()> {
    CrateOwner::builder()
        .crate_id(krate.id)
        .team_id(t.id)
        .created_by(u.id)
        .build()
        .insert(conn)
        .await
}
