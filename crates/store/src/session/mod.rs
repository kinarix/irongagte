use async_trait::async_trait;
use fred::prelude::*;
use irongate_core::{
    errors::StoreError,
    repositories::{AuthCodeData, AuthCodeStore, SessionRepository},
    Session,
};
use uuid::Uuid;

pub struct RedisSessionStore {
    client: Client,
}

impl RedisSessionStore {
    fn auth_code_key(code: &str) -> String {
        format!("authcode:{code}")
    }

    pub async fn new(url: &str) -> Result<Self, StoreError> {
        let config = Config::from_url(url).map_err(|e| StoreError::Cache(e.to_string()))?;
        let client = Builder::from_config(config)
            .build()
            .map_err(|e| StoreError::Cache(e.to_string()))?;
        client.init().await.map_err(|e| StoreError::Cache(e.to_string()))?;
        Ok(Self { client })
    }

    fn session_key(id: Uuid) -> String {
        format!("session:{id}")
    }

    fn user_sessions_key(tenant_id: Uuid, user_id: Uuid) -> String {
        format!("user_sessions:{tenant_id}:{user_id}")
    }

    fn map_err(e: fred::error::Error) -> StoreError {
        StoreError::Cache(e.to_string())
    }

    fn ttl_secs(session: &Session) -> i64 {
        let remaining = session.expires_at - time::OffsetDateTime::now_utc();
        remaining.whole_seconds().max(1)
    }
}

#[async_trait]
impl SessionRepository for RedisSessionStore {
    async fn create(&self, session: Session) -> Result<Session, StoreError> {
        let key = Self::session_key(session.id);
        let user_key = Self::user_sessions_key(session.tenant_id, session.user_id);
        let ttl = Self::ttl_secs(&session);

        let json = serde_json::to_string(&session)
            .map_err(|e| StoreError::Cache(e.to_string()))?;

        let _: () = self
            .client
            .set(&key, json, Some(Expiration::EX(ttl)), None, false)
            .await
            .map_err(Self::map_err)?;

        let _: i64 = self
            .client
            .sadd(&user_key, session.id.to_string())
            .await
            .map_err(Self::map_err)?;

        Ok(session)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Session, StoreError> {
        let key = Self::session_key(id);
        let json: Option<String> = self.client.get(&key).await.map_err(Self::map_err)?;

        match json {
            Some(j) => serde_json::from_str(&j).map_err(|e| StoreError::Cache(e.to_string())),
            None => Err(StoreError::NotFound(format!("session {id}"))),
        }
    }

    async fn revoke(&self, id: Uuid) -> Result<(), StoreError> {
        let key = Self::session_key(id);
        let json: Option<String> = self.client.get(&key).await.map_err(Self::map_err)?;

        let json = match json {
            Some(j) => j,
            None => return Err(StoreError::NotFound(format!("session {id}"))),
        };

        let mut session: Session =
            serde_json::from_str(&json).map_err(|e| StoreError::Cache(e.to_string()))?;
        session.revoked_at = Some(time::OffsetDateTime::now_utc());

        let updated = serde_json::to_string(&session)
            .map_err(|e| StoreError::Cache(e.to_string()))?;
        let ttl = Self::ttl_secs(&session);

        let _: () = self
            .client
            .set(&key, updated, Some(Expiration::EX(ttl)), None, false)
            .await
            .map_err(Self::map_err)?;

        let user_key = Self::user_sessions_key(session.tenant_id, session.user_id);
        let _: i64 = self
            .client
            .srem(&user_key, id.to_string())
            .await
            .map_err(Self::map_err)?;

        Ok(())
    }

    async fn revoke_all_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<u64, StoreError> {
        let user_key = Self::user_sessions_key(tenant_id, user_id);
        let session_ids: Vec<String> =
            self.client.smembers(&user_key).await.map_err(Self::map_err)?;

        let count = session_ids.len() as u64;
        let now = time::OffsetDateTime::now_utc();

        for id_str in &session_ids {
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let key = Self::session_key(id);
            let json: Option<String> = self.client.get(&key).await.map_err(Self::map_err)?;
            if let Some(j) = json {
                if let Ok(mut session) = serde_json::from_str::<Session>(&j) {
                    session.revoked_at = Some(now);
                    if let Ok(updated) = serde_json::to_string(&session) {
                        let ttl = Self::ttl_secs(&session);
                        let _: () = self
                            .client
                            .set(&key, updated, Some(Expiration::EX(ttl)), None, false)
                            .await
                            .map_err(Self::map_err)?;
                    }
                }
            }
        }

        let _: i64 = self.client.del(&user_key).await.map_err(Self::map_err)?;

        Ok(count)
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Session>, StoreError> {
        let user_key = Self::user_sessions_key(tenant_id, user_id);
        let session_ids: Vec<String> =
            self.client.smembers(&user_key).await.map_err(Self::map_err)?;

        let mut sessions = Vec::with_capacity(session_ids.len());
        for id_str in &session_ids {
            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let key = Self::session_key(id);
            let json: Option<String> = self.client.get(&key).await.map_err(Self::map_err)?;
            if let Some(j) = json {
                if let Ok(session) = serde_json::from_str::<Session>(&j) {
                    sessions.push(session);
                }
            }
        }

        Ok(sessions)
    }
}

#[async_trait]
impl AuthCodeStore for RedisSessionStore {
    async fn store_code(
        &self,
        code: &str,
        data: AuthCodeData,
        ttl_secs: i64,
    ) -> Result<(), StoreError> {
        let key = Self::auth_code_key(code);
        let json =
            serde_json::to_string(&data).map_err(|e| StoreError::Cache(e.to_string()))?;
        let _: () = self
            .client
            .set(&key, json, Some(Expiration::EX(ttl_secs)), None, false)
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn take_code(&self, code: &str) -> Result<Option<AuthCodeData>, StoreError> {
        let key = Self::auth_code_key(code);
        let json: Option<String> = self.client.getdel(&key).await.map_err(Self::map_err)?;
        match json {
            None => Ok(None),
            Some(j) => {
                let data = serde_json::from_str(&j)
                    .map_err(|e| StoreError::Cache(e.to_string()))?;
                Ok(Some(data))
            }
        }
    }
}
