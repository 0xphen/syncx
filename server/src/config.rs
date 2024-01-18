use dotenv::dotenv;

use crate::errors::SynxServerError;

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    pub db_name: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_exp: i64,
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

        let db_name = std::env::var("REDIS_URL").map_err(|_err| {
            SynxServerError::InvalidServerSettings("DB_NAME not present".to_string())
        })?;

        let jwt_secret = std::env::var("JWT_SECRET").map_err(|_err| {
            SynxServerError::InvalidServerSettings("JWT_SECRET not present".to_string())
        })?;

        let mut jwt_exp = std::env::var("JWT_EXP").map_err(|_err| {
            SynxServerError::InvalidServerSettings("JWT_EXP not present".to_string())
        })?;
        let jwt_exp = jwt_exp
            .parse::<i64>()
            .map_err(|_err| SynxServerError::ParseIntError)?;

        Ok(Config {
            database_url,
            redis_url,
            db_name,
            jwt_secret,
            jwt_exp,
        })
    }
}
