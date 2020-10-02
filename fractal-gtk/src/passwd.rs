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

impl From<secret_service::SsError> for Error {
    fn from(_: secret_service::SsError) -> Error {
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
    fn delete_pass(&self, key: &str) -> Result<(), Error> {
        ss_storage::delete_pass(key)
    }

    fn store_pass(
        &self,
        username: String,
        password: String,
        server: Url,
        identity: Box<ServerName>,
    ) -> Result<(), Error> {
        ss_storage::store_pass(username, password, server, identity)
    }

    fn get_pass(&self) -> Result<(String, String, Url, Box<ServerName>), Error> {
        ss_storage::get_pass()
    }

    fn store_token(&self, uid: UserId, token: AccessToken) -> Result<(), Error> {
        ss_storage::store_token(uid, token)
    }

    fn get_token(&self) -> Result<(Option<AccessToken>, UserId), Error> {
        ss_storage::get_token()
    }
}

mod ss_storage {
    use super::Error;
    use crate::api::r0::AccessToken;
    use matrix_sdk::identifiers::{ServerName, UserId};
    use std::convert::{TryFrom, TryInto};
    use url::Url;

    use secret_service::{Collection, EncryptionType, SecretService, SsError};

    use crate::globals;

    pub fn delete_pass(key: &str) -> Result<(), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = get_default_collection_unlocked(&ss)?;

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

    pub fn store_token(uid: UserId, token: AccessToken) -> Result<(), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = get_default_collection_unlocked(&ss)?;
        let key = "fractal-token";

        // deleting previous items
        delete_pass(key)?;

        // create new item
        collection.create_item(
            key,                             // label
            vec![("uid", &uid.to_string())], // properties
            token.to_string().as_bytes(),    // secret
            true,                            // replace item with same attributes
            "text/plain",                    // secret content type
        )?;

        Ok(())
    }

    pub fn get_token() -> Result<(Option<AccessToken>, UserId), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = get_default_collection_unlocked(&ss)?;
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
    ) -> Result<(), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = get_default_collection_unlocked(&ss)?;
        let key = "fractal";

        // deleting previous items
        delete_pass(key)?;

        // create new item
        collection.create_item(
            key, // label
            vec![
                ("username", &username),
                ("server", server.as_str()),
                ("identity", identity.as_str()),
            ], // properties
            password.as_bytes(), //secret
            true, // replace item with same attributes
            "text/plain", // secret content type
        )?;

        Ok(())
    }

    pub fn migrate_old_passwd() -> Result<(), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = get_default_collection_unlocked(&ss)?;
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

        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = get_default_collection_unlocked(&ss)?;
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

    fn get_default_collection_unlocked(
        secret_service: &SecretService,
    ) -> Result<Collection, secret_service::SsError> {
        let collection = match secret_service.get_default_collection() {
            Ok(col) => col,
            Err(SsError::NoResult) => secret_service.create_collection("default", "default")?,
            Err(x) => return Err(x),
        };

        collection.unlock()?;

        Ok(collection)
    }
}
