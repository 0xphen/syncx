pub mod jwt {
    use chrono::Utc;
    use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};

    use crate::errors::SynxServerError;

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iss: String,
    }

    pub fn create_jwt(uid: &str, secret: &str, t: i64) -> Result<String, SynxServerError> {
        let mut header = Header::new(Algorithm::HS512);
        header.typ = Some("JWT".to_string());

        let exp = Utc::now()
            .checked_add_signed(chrono::Duration::seconds(t))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            iss: "SyncxServer".to_string(),
            sub: uid.to_string(),
            exp,
        };

        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|_| SynxServerError::JWTTokenCreationError)
    }
}

pub mod hash_utils {
    use argon2::{
        password_hash::{
            rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        },
        Argon2,
    };

    use crate::errors::SynxServerError;

    pub fn hash_password(password: &str) -> Result<String, SynxServerError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        Ok(argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| SynxServerError::PasswordHashError)?
            .to_string())
    }
}
