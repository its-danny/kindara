use sqlx::{types::Json, Postgres, Transaction};

use crate::world::resources::WorldState;

pub async fn store_world_state<'a>(
    state: &WorldState,
    transaction: &mut Transaction<'a, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO world_saves (state) VALUES ($1)")
        .bind(Json(&state))
        .execute(&mut **transaction)
        .await?;

    sqlx::query("DELETE FROM world_saves WHERE id IN (SELECT id FROM world_saves ORDER BY id ASC LIMIT 1) AND (SELECT COUNT(*) FROM world_saves) > 10080")
            .execute(&mut **transaction)
            .await?;

    Ok(())
}
