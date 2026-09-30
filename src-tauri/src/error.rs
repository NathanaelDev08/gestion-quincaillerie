use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Erreur de base de données: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Erreur d'authentification: {0}")]
    Auth(String),

    #[error("Utilisateur non trouvé")]
    UserNotFound,

    #[error("Mot de passe incorrect")]
    InvalidPassword,

    #[error("Token invalide ou expiré")]
    InvalidToken,

    #[error("Permission refusée: {0}")]
    Forbidden(String),

    #[error("Validation échouée: {0}")]
    Validation(String),

    #[error("Ressource non trouvée: {0}")]
    NotFound(String),

    #[error("Conflit: {0}")]
    Conflict(String),

    #[error("Erreur de sérialisation: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Erreur IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur PDF: {0}")]
    Pdf(String),

    #[error("Erreur de configuration: {0}")]
    Config(String),

    #[error("Erreur interne: {0}")]
    Internal(String),
}

impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        err.to_string()
    }
}

pub type AppResult<T> = Result<T, AppError>;