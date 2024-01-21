use dotenv::dotenv;

use super::errors::SynxServerError;

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_exp: i64,
    pub db_name: String,
    pub gcs_bucket_name: String,
}

impl Config {
    pub fn load_config() -> Result<Config, SynxServerError> {
        dotenv().ok();

        let database_url = std::env::var("DATABASE_URL").map_err(|_err| {
            SynxServerError::InvalidServerSettings("DATABASE_URL not present".to_string())
        })?;

        let redis_url = std::env::var("REDIS_URL").map_err(|_err| {
            SynxServerError::InvalidServerSettings("REDIS_URL not present".to_string())
        })?;

        let jwt_secret = std::env::var("JWT_SECRET").map_err(|_err| {
            SynxServerError::InvalidServerSettings("JWT_SECRET not present".to_string())
        })?;

        let jwt_exp = std::env::var("JWT_EXP").map_err(|_err| {
            SynxServerError::InvalidServerSettings("JWT_EXP not present".to_string())
        })?;

        let jwt_exp = jwt_exp
            .parse::<i64>()
            .map_err(|_err| SynxServerError::ParseIntError)?;

        let db_name = std::env::var("DB_NAME").map_err(|_err| {
            SynxServerError::InvalidServerSettings("DB_NAME not present".to_string())
        })?;

        let gcs_bucket_name = std::env::var("GCS_BUCKET_NAME").map_err(|_err| {
            SynxServerError::InvalidServerSettings("GCS_BUCKET_NAME not present".to_string())
        })?;

        std::env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON").map_err(|_err| {
            SynxServerError::InvalidServerSettings(
                "GOOGLE_APPLICATION_CREDENTIALS_JSON not present".to_string(),
            )
        })?;

        std::env::var("GOOGLE_STORAGE_API_KEY").map_err(|_err| {
            SynxServerError::InvalidServerSettings("GOOGLE_STORAGE_API_KEY not present".to_string())
        })?;

        Ok(Config {
            database_url,
            redis_url,
            jwt_secret,
            jwt_exp,
            gcs_bucket_name,
            db_name,
        })
    }
}
