use crate::api::r0::AccessToken;
use matrix_sdk::identifiers::{Error as IdError, ServerName, UserId};
use url::ParseError;
use url::Url;

#[derive(Debug)]
pub enum Error {
    SecretServiceError,
    UrlParseError(ParseError),
    IdParseError(IdError),
}

impl From<secret_service::Error> for Error {
    fn from(_: secret_service::Error) -> Error {
        Error::SecretServiceError
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::UrlParseError(err)
    }
}

impl From<IdError> for Error {
    fn from(err: IdError) -> Error {
        Error::IdParseError(err)
    }
}

pub trait PasswordStorage {
    fn delete_secret(&self, key: &str) -> Result<(), secret_service::Error> {
        ss_storage::delete_secret(key)
    }

    fn store_pass(
        &self,
        username: String,
        password: String,
        server: Url,
        identity: Box<ServerName>,
    ) -> Result<(), secret_service::Error> {
        ss_storage::store_pass(username, password, server, identity)
    }

    fn get_pass(&self) -> Result<(String, String, Url, Box<ServerName>), Error> {
        ss_storage::get_pass()
    }

    fn store_token(&self, uid: UserId, token: AccessToken) -> Result<(), secret_service::Error> {
        ss_storage::store_token(uid, token)
    }

    fn get_token(&self) -> Result<(Option<AccessToken>, UserId), Error> {
        ss_storage::get_token()
    }
}

mod ss_storage {
    use std::collections::HashMap;
    use std::convert::{TryFrom, TryInto};

    use matrix_sdk::identifiers::{ServerName, UserId};
    use once_cell::sync::Lazy;
    use secret_service::{Collection, EncryptionType, Error as SsError, SecretService};
    use url::Url;

    use super::Error;
    use crate::api::r0::AccessToken;
    use crate::globals;

    static SECRET_SERVICE: Lazy<Result<SecretService<'static>, SsError>> =
        Lazy::new(|| SecretService::new(EncryptionType::Dh));

    pub fn delete_secret(key: &str) -> Result<(), SsError> {
        let collection = get_default_collection_unlocked()?;

        // deleting previous items
        let allpass = collection.get_all_items()?;
        let passwds = allpass
            .iter()
            .filter(|x| x.get_label().unwrap_or_default() == key);
        for p in passwds {
            p.unlock()?;
            p.delete()?;
        }

        Ok(())
    }

    pub fn store_token(uid: UserId, token: AccessToken) -> Result<(), SsError> {
        let collection = get_default_collection_unlocked()?;
        let key = "fractal-token";

        // deleting previous items
        delete_secret(key)?;

        // create new item
        let mut attributes = HashMap::new();
        attributes.insert("uid", uid.as_str());
        collection.create_item(
            key,                          // label
            attributes,                   // properties
            token.to_string().as_bytes(), // secret
            true,                         // replace item with same attributes
            "text/plain",                 // secret content type
        )?;

        Ok(())
    }

    pub fn get_token() -> Result<(Option<AccessToken>, UserId), Error> {
        let collection = get_default_collection_unlocked()?;
        let allpass = collection.get_all_items()?;
        let key = "fractal-token";

        let passwd = allpass
            .iter()
            .find(|x| x.get_label().unwrap_or_default() == key);

        if passwd.is_none() {
            return Err(Error::SecretServiceError);
        }

        let p = passwd.unwrap();
        p.unlock()?;
        let attrs = p.get_attributes()?;
        let secret = p.get_secret()?;
        let token = Some(String::from_utf8(secret).unwrap())
            .filter(|tk| !tk.is_empty())
            .map(AccessToken::from);

        let attr = attrs
            .iter()
            .find(|x| x.0 == "uid")
            .ok_or(Error::SecretServiceError)?;
        let uid = UserId::try_from(attr.1.as_str())?;

        Ok((token, uid))
    }

    pub fn store_pass(
        username: String,
        password: String,
        server: Url,
        identity: Box<ServerName>,
    ) -> Result<(), SsError> {
        let collection = get_default_collection_unlocked()?;
        let key = "fractal";

        // deleting previous items
        delete_secret(key)?;

        // create new item
        let mut attributes = HashMap::new();
        attributes.insert("username", username.as_str());
        attributes.insert("server", server.as_str());
        attributes.insert("identity", identity.as_str());
        collection.create_item(
            key,                 // label
            attributes,          // properties
            password.as_bytes(), //secret
            true,                // replace item with same attributes
            "text/plain",        // secret content type
        )?;

        Ok(())
    }

    pub fn migrate_old_passwd() -> Result<(), Error> {
        let collection = get_default_collection_unlocked()?;
        let allpass = collection.get_all_items()?;

        // old name password
        let passwd = allpass
            .iter()
            .find(|x| x.get_label().unwrap_or_default() == "guillotine");

        if passwd.is_none() {
            return Ok(());
        }

        let p = passwd.unwrap();
        p.unlock()?;
        let attrs = p.get_attributes()?;
        let secret = p.get_secret()?;

        let mut attr = attrs
            .iter()
            .find(|x| x.0 == "username")
            .ok_or(Error::SecretServiceError)?;
        let username = attr.1.clone();
        attr = attrs
            .iter()
            .find(|x| x.0 == "server")
            .ok_or(Error::SecretServiceError)?;
        let server = Url::parse(&attr.1)?;
        let pwd = String::from_utf8(secret).unwrap();

        // removing old
        if let Some(p) = passwd {
            p.delete()?;
        }
        /* Fallback to default identity server if there is none */
        let identity = globals::DEFAULT_IDENTITYSERVER.clone();

        store_pass(username, pwd, server, identity)?;

        Ok(())
    }

    pub fn get_pass() -> Result<(String, String, Url, Box<ServerName>), Error> {
        migrate_old_passwd()?;

        let collection = get_default_collection_unlocked()?;
        let allpass = collection.get_all_items()?;
        let key = "fractal";

        let passwd = allpass
            .iter()
            .find(|x| x.get_label().unwrap_or_default() == key);

        if passwd.is_none() {
            return Err(Error::SecretServiceError);
        }

        let p = passwd.unwrap();
        p.unlock()?;
        let attrs = p.get_attributes()?;
        let secret = p.get_secret()?;

        let attr = attrs
            .iter()
            .find(|x| x.0 == "username")
            .ok_or(Error::SecretServiceError)?;
        let username = attr.1.clone();
        let attr = attrs
            .iter()
            .find(|x| x.0 == "server")
            .ok_or(Error::SecretServiceError)?;
        let server = Url::parse(&attr.1)?;

        let attr = attrs.iter().find(|x| x.0 == "identity");

        /* Fallback to the vector identity server when there is none */
        let identity = match attr {
            Some(ref a) => Url::parse(a.1.as_str())
                .map_err(Error::from)
                .and_then(|u| {
                    u.host_str()
                        .unwrap_or_default()
                        .try_into()
                        .map_err(Error::from)
                })
                .or_else(|_| a.1.as_str().try_into().map_err(Error::from))?,
            None => globals::DEFAULT_IDENTITYSERVER.clone(),
        };

        let tup = (
            username,
            String::from_utf8(secret).unwrap(),
            server,
            identity,
        );

        Ok(tup)
    }

    fn get_default_collection_unlocked<'a>() -> Result<Collection<'a>, SsError> {
        if SECRET_SERVICE.is_ok() {
            let ss = SECRET_SERVICE.as_ref().unwrap();
            let collection = match ss.get_default_collection() {
                Ok(col) => col,
                Err(SsError::NoResult) => ss.create_collection("default", "default")?,
                Err(x) => return Err(x),
            };

            collection.unlock()?;

            Ok(collection)
        } else {
            Err(SsError::Crypto(
                "Error encountered when initiating secret service connection.".to_string(),
            ))
        }
    }
}
