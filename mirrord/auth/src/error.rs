pub type AuthenticationError = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T, E = AuthenticationError> = std::result::Result<T, E>;
