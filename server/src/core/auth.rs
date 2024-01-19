pub mod jwt {
    use chrono::Utc;
    use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};

    use crate::core::errors::SynxServerError;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Claims {
        sub: String,
        exp: usize,
        iss: String,
    }
    /// Creates a JSON Web Token (JWT) for a given user identifier using the HS512 signing algorithm.
    ///
    /// The HS512 algorithm uses a provided secret key to create a signature with the HMAC (Hash-based
    /// Message Authentication Code) method using SHA-512. The `iss` (issuer) claim is set to "SyncxServer",
    /// and the `sub` (subject) claim includes the user identifier. The token's expiration (`exp`) is set to
    /// the current time plus the specified duration `t`.
    ///
    /// # Arguments
    /// * `uid` - The unique identifier for the user.
    /// * `secret` - The secret key used for signing the token.
    /// * `t` - The lifespan of the token in seconds from the current time.
    ///
    /// # Returns
    /// A `Result` which is either:
    /// - `Ok(String)`: A string representation of the JWT upon successful creation.
    /// - `Err(SynxServerError)`: An error wrapped in `SynxServerError` when token creation fails.
    ///
    /// # Errors
    /// Returns an error if the token cannot be created, which may occur if:
    /// - The secret key is invalid.
    /// - There is an issue with the token's payload or header.
    /// - The system time cannot be retrieved or computed into an expiration timestamp.
    pub fn create_jwt(uid: &str, secret: &str, t: i64) -> Result<String, SynxServerError> {
        let mut header = Header::new(Algorithm::HS512);

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

    pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, SynxServerError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS512),
        )
        .map_err(|_| SynxServerError::InvalidJWTTokenError)?;

        Ok(token_data.claims)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{thread::sleep, time::Duration};

        const SECRET: &'static str = "secret";

        #[test]
        fn jwt_should_be_valid() {
            let jwt = create_jwt("uid", SECRET, 60).unwrap();
            let claims = verify_jwt(&jwt, SECRET).unwrap();

            assert!(claims.sub == "uid".to_string());
            assert!(claims.iss == "SyncxServer".to_string());
            assert!(claims.exp > Utc::now().timestamp() as usize);
        }

        #[test]
        fn jwt_should_expire() {
            let jwt = create_jwt("uid", SECRET, 1).unwrap();

            // Sleep for longer than the token's expiration time
            sleep(Duration::from_secs(2));

            let claims = verify_jwt(&jwt, SECRET).unwrap();
            assert!(claims.exp < Utc::now().timestamp() as usize);
        }
    }
}

pub mod hash_utils {
    use argon2::{
        password_hash::{
            rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        },
        Argon2,
    };

    use crate::core::errors::SynxServerError;

    pub fn hash_password(password: &str) -> Result<String, SynxServerError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        Ok(argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| SynxServerError::PasswordHashError)?
            .to_string())
    }
}
