use irongate_core::errors::StoreError;

pub(crate) fn map_db_err(e: sqlx::Error) -> StoreError {
    match e {
        sqlx::Error::RowNotFound => StoreError::NotFound("record not found".into()),
        sqlx::Error::Database(ref db) if db.is_unique_violation() => {
            StoreError::Conflict(e.to_string())
        }
        _ => StoreError::Database(e.to_string()),
    }
}

pub(crate) fn map_row_err(e: sqlx::Error) -> StoreError {
    StoreError::Database(e.to_string())
}

pub(crate) fn map_json_err(field: &'static str) -> impl Fn(serde_json::Error) -> StoreError {
    move |e| StoreError::Database(format!("invalid json in {field}: {e}"))
}

pub(crate) fn map_parse_err(field: &'static str) -> StoreError {
    StoreError::Database(format!("unparseable value in field: {field}"))
}
