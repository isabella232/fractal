use either::Either;
use fractal_api::api::r0::user_directory::search_users::User;
use fractal_api::identifiers::UserId;
use fractal_api::r0::sync::get_joined_members::RoomMember;
use fractal_api::url::{ParseError as UrlError, Url};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

// TODO: Make this non-(de)serializable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub uid: UserId,
    #[serde(rename = "display_name")]
    pub alias: Option<String>,
    #[serde(rename = "avatar_url")]
    pub avatar: Option<Either<Url, PathBuf>>,
}

impl Member {
    pub fn get_alias(&self) -> String {
        if let Some(ref alias) = self.alias {
            if !alias.is_empty() {
                return alias.clone();
            }
        }
        self.uid.to_string()
    }
}

impl PartialEq for Member {
    fn eq(&self, other: &Member) -> bool {
        self.uid == other.uid
    }
}

impl TryFrom<User> for Member {
    type Error = UrlError;

    fn try_from(user: User) -> Result<Self, Self::Error> {
        Ok(Self {
            uid: user.user_id,
            alias: user.display_name,
            avatar: user
                .avatar_url
                .filter(|a| !a.is_empty())
                .map(|url| Url::parse(&url))
                .transpose()?
                .map(Either::Left),
        })
    }
}

impl TryFrom<(UserId, RoomMember)> for Member {
    type Error = UrlError;

    fn try_from((uid, roommember): (UserId, RoomMember)) -> Result<Self, Self::Error> {
        Ok(Member {
            uid,
            alias: roommember.display_name,
            avatar: roommember
                .avatar_url
                .filter(|url| !url.is_empty())
                .map(|url| Url::parse(&url))
                .transpose()?
                .map(Either::Left),
        })
    }
}

// hashmap userid -> Member
pub type MemberList = HashMap<UserId, Member>;
