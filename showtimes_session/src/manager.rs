//! The manager for session, powered via Redis/Valkey

use std::sync::Arc;

use jsonwebtoken::errors::ErrorKind;
use redis::cmd;
use redis::AsyncCommands;
use redis::RedisResult;

use crate::ShowtimesEncodingKey;

use super::{
    verify_refresh_session, verify_session, ShowtimesAudience, ShowtimesRefreshSession,
    ShowtimesUserClaims, ShowtimesUserSession,
};

/// The shared [`SessionManager`] instance for the showtimes service.
///
/// Can be used between threads safely.
pub type SharedSessionManager = std::sync::Arc<tokio::sync::Mutex<SessionManager>>;
const SESSION_MANAGER: &str = "showtimes:session";
const SESSION_REFRESH_MANAGER: &str = "showtimes:session:refresh";

/// Redis-managed session state for the showtimes service.
#[derive(Debug, Clone)]
pub struct SessionManager {
    connection: redis::aio::MultiplexedConnection,
    secret: Arc<ShowtimesEncodingKey>,
}

/// The kind of session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// Bearer type, a.k.a using OAuth2 token
    Bearer,
    /// API key authentication
    APIKey,
    /// Master key authentication, set via config file
    MasterKey,
}

/// Error type for session manager.
#[derive(Debug)]
pub enum SessionError {
    /// The session is invalid.
    InvalidSession,
    /// The session has invalid signature.
    InvalidSignature,
    /// The session has invalid format
    InvalidSessionFormat,
    /// The session is expired.
    ExpiredSession,
    /// Session not found
    SessionNotFound,
    /// An error from redis
    RedisError(redis::RedisError),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSession => write!(f, "Invalid session"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InvalidSessionFormat => write!(f, "Invalid session format"),
            Self::ExpiredSession => write!(f, "Expired session"),
            Self::SessionNotFound => write!(f, "Session not found"),
            Self::RedisError(e) => write!(f, "Redis error: {}", e),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<SessionKind> for ShowtimesAudience {
    fn from(value: SessionKind) -> Self {
        match value {
            SessionKind::Bearer => ShowtimesAudience::User,
            SessionKind::APIKey => ShowtimesAudience::APIKey,
            SessionKind::MasterKey => ShowtimesAudience::MasterKey,
        }
    }
}

impl SessionManager {
    /// Create a new session manager.
    pub async fn new(
        client: &Arc<redis::Client>,
        secret: &Arc<ShowtimesEncodingKey>,
    ) -> RedisResult<Self> {
        let client_name = format!("showtimes-rs/{}", env!("CARGO_PKG_VERSION"));

        let mut con = client.get_multiplexed_async_connection().await?;
        // Test the connection
        cmd("PING").exec_async(&mut con).await?;

        // Set the client name
        cmd("CLIENT")
            .arg("SETNAME")
            .arg(client_name)
            .exec_async(&mut con)
            .await?;

        Ok(Self {
            connection: con,
            secret: secret.clone(),
        })
    }

    /// Get reference to the internal [`ShowtimesEncodingKey`]
    pub fn get_secret(&self) -> &ShowtimesEncodingKey {
        &self.secret
    }

    /// Delete a session from the session manager.
    pub async fn remove_session(&mut self, token: impl Into<String>) -> RedisResult<()> {
        self.connection.hdel(SESSION_MANAGER, token.into()).await
    }

    /// Delete a refresh session from the session manager.
    pub async fn remove_refresh_session(&mut self, token: impl Into<String>) -> RedisResult<()> {
        self.connection
            .hdel(SESSION_REFRESH_MANAGER, token.into())
            .await
    }

    /// Get a session from the session manager.
    ///
    /// Then, verify the session.
    pub async fn get_session(
        &mut self,
        token: impl Into<String>,
        kind: SessionKind,
    ) -> Result<ShowtimesUserSession, SessionError> {
        let token: String = token.into();

        match kind {
            SessionKind::APIKey | SessionKind::MasterKey => {
                let session = ShowtimesUserClaims::new_api(token.clone(), kind.into());

                Ok(ShowtimesUserSession::new(token, session))
            }
            SessionKind::Bearer => {
                // We use hashmaps to store the session data
                let session_exp: Option<i64> = self
                    .connection
                    .hget(SESSION_MANAGER, &token)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to get session: {:?}", e);
                        SessionError::RedisError(e)
                    })?;

                match session_exp {
                    None => Err(SessionError::SessionNotFound),
                    Some(session_exp) => {
                        if session_exp != -1 {
                            let current_time = chrono::Utc::now() - chrono::Duration::minutes(2);

                            if session_exp < current_time.timestamp() {
                                // Delete the session
                                self.remove_session(&token).await.map_err(|e| {
                                    tracing::error!("Failed to remove session: {:?}", e);
                                    SessionError::RedisError(e)
                                })?;
                                return Err(SessionError::ExpiredSession);
                            }
                        }

                        let session_res = verify_session(&token, &self.secret, kind.into())
                            .map_err(|e| {
                                tracing::error!("Failed to verify session: {:?}", e);
                                match e.kind() {
                                    ErrorKind::ExpiredSignature => SessionError::ExpiredSession,
                                    ErrorKind::InvalidSignature => SessionError::InvalidSignature,
                                    _ => SessionError::InvalidSession,
                                }
                            });

                        match session_res {
                            Ok(session) => Ok(ShowtimesUserSession::new(token, session)),
                            Err(SessionError::ExpiredSession) => {
                                // Delete the session
                                self.remove_session(&token).await.map_err(|e| {
                                    tracing::error!("Failed to remove session: {:?}", e);
                                    SessionError::RedisError(e)
                                })?;
                                Err(SessionError::ExpiredSession)
                            }
                            Err(e) => Err(e),
                        }
                    }
                }
            }
        }
    }

    /// Set a session to the session manager.
    pub async fn set_session(
        &mut self,
        token: impl Into<String>,
        session: &ShowtimesUserClaims,
    ) -> RedisResult<()> {
        let token: String = token.into();
        let session_exp = session.exp;

        self.connection
            .hset(SESSION_MANAGER, token, session_exp.to_string())
            .await
    }

    /// Get a refresh token information
    ///
    /// Returns the refresh token information and the current token saved.
    pub async fn get_refresh_session(
        &mut self,
        token: impl Into<String>,
    ) -> Result<(ShowtimesRefreshSession, String), SessionError> {
        let token: String = token.into();

        let token_session: Option<String> = self
            .connection
            .hget(SESSION_REFRESH_MANAGER, &token)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get refresh session: {:?}", e);
                SessionError::RedisError(e)
            })?;

        match token_session {
            None => Err(SessionError::SessionNotFound),
            Some(token_session) => {
                let refresh_res = verify_refresh_session(&token, &self.secret).map_err(|e| {
                    tracing::error!("Failed to verify refresh session: {:?}", e);
                    match e.kind() {
                        ErrorKind::ExpiredSignature => SessionError::ExpiredSession,
                        ErrorKind::InvalidSignature => SessionError::InvalidSignature,
                        _ => SessionError::InvalidSession,
                    }
                });

                match refresh_res {
                    Ok(refresh_res) => Ok((
                        ShowtimesRefreshSession::new(&token, refresh_res),
                        token_session,
                    )),
                    Err(SessionError::ExpiredSession) => {
                        // Delete the session
                        self.remove_refresh_session(&token).await.map_err(|e| {
                            tracing::error!("Failed to remove refreshsession: {:?}", e);
                            SessionError::RedisError(e)
                        })?;
                        Err(SessionError::ExpiredSession)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    /// Set a refresh session to the session manager.
    pub async fn set_refresh_session(
        &mut self,
        refresh_token: impl Into<String>,
        session_token: impl Into<String>,
    ) -> RedisResult<()> {
        let refresh_token: String = refresh_token.into();
        let session_token: String = session_token.into();

        let token_session: Option<String> = self
            .connection
            .hget(SESSION_REFRESH_MANAGER, &refresh_token)
            .await?;

        if let Some(token_session) = token_session {
            // Remove old session
            self.remove_session(&token_session).await?;
        }

        self.connection
            .hset(SESSION_REFRESH_MANAGER, refresh_token, session_token)
            .await
    }
}
