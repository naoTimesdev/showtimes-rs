use jsonwebtoken::errors::ErrorKind;
use redis::cmd;
use redis::AsyncCommands;
use redis::RedisResult;

use super::{verify_session, ShowtimesAudience, ShowtimesUserClaims, ShowtimesUserSession};

pub type SharedSessionManager = std::sync::Arc<tokio::sync::Mutex<SessionManager>>;
const SESSION_MANAGER: &str = "showtimes:session";

/// Redis-managed session state for the showtimes service.
#[derive(Debug, Clone)]
pub struct SessionManager {
    connection: redis::aio::MultiplexedConnection,
    secret: String,
}

/// The kind of session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Bearer,
    APIKey,
    MasterKey,
}

/// Error type for session manager.
#[derive(Debug)]
pub enum SessionError {
    /// The session is invalid.
    InvalidSession,
    /// The session has invalid signature.
    InvalidSignature,
    /// The session is expired.
    ExpiredSession,
    SessionNotFound,
    /// An error from redis
    RedisError(redis::RedisError),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSession => write!(f, "Invalid session"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::ExpiredSession => write!(f, "Expired session"),
            Self::SessionNotFound => write!(f, "Session not found"),
            Self::RedisError(e) => write!(f, "Redis error: {}", e),
        }
    }
}

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
    pub async fn new(url: impl Into<String>, secret: impl Into<String>) -> RedisResult<Self> {
        let client_name = format!("showtimes-rs/{}", env!("CARGO_PKG_VERSION"));
        let client = redis::Client::open(url.into()).unwrap();

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
            secret: secret.into(),
        })
    }

    pub async fn remove_session(&mut self, token: impl Into<String>) -> RedisResult<()> {
        self.connection.hdel(SESSION_MANAGER, token.into()).await
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
        session: ShowtimesUserClaims,
    ) -> RedisResult<()> {
        let token: String = token.into();
        let session_exp = session.exp;

        self.connection
            .hset(SESSION_MANAGER, token, session_exp.to_string())
            .await
    }
}
