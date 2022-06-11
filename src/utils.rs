#![allow(dead_code)]

use std::borrow::Cow;
use std::fmt::Display;

use serde::Serialize;
use twilight_http::request::application::interaction::{CreateFollowup, UpdateResponse};
use twilight_http::request::channel::message::{
    CreateMessage, GetChannelMessagesConfigured, GetMessage, UpdateMessage,
};
use twilight_http::request::channel::GetChannel;
use twilight_http::request::guild::emoji::GetEmojis;
use twilight_http::request::guild::member::GetMember;
use twilight_http::request::guild::role::GetGuildRoles;
use twilight_http::request::guild::GetGuild;
use twilight_http::request::user::{GetCurrentUser, GetCurrentUserGuildMember, GetUser};
use twilight_http::request::GetUserApplicationInfo;
use twilight_model::channel::{Channel, Message, ReactionType};
use twilight_model::guild::{Emoji, Guild, Member, Role};
use twilight_model::id::marker::EmojiMarker;
use twilight_model::id::Id;
use twilight_model::oauth::Application;
use twilight_model::user::{CurrentUser, User};

pub use crate::utils::prelude::*;

/// Re-exports of useful things.
pub mod prelude {
    pub use anyhow::Result as AnyResult;
    pub use async_trait::async_trait;
    pub use tracing::{debug, error, info, trace, warn};

    pub use super::{impl_debug_struct_fields, ExecModelExt};
}

/// Universal constants.
pub mod consts {
    pub const EVERYONE: &str = "@everyone";
    pub const DELIMITERS: &[char] = &['\'', '"', '`'];
}

/// A trait to simplify `.exec().await?.model.await` chain.
#[async_trait]
pub trait ExecModelExt {
    type Value;

    /// Send the command by calling `exec()` and `model()`.
    async fn send(self) -> AnyResult<Self::Value>;
}

/// Macro to implement `ExecModelExt` in a one-liner.
macro impl_exec_model_ext($req:ty, $val:ty) {
    #[async_trait]
    impl ExecModelExt for $req {
        type Value = $val;

        async fn send(self) -> AnyResult<Self::Value> {
            self.exec().await?.model().await.map_err(Into::into)
        }
    }
}

impl_exec_model_ext!(CreateFollowup<'_>, Message);
impl_exec_model_ext!(CreateMessage<'_>, Message);
impl_exec_model_ext!(GetChannel<'_>, Channel);
impl_exec_model_ext!(GetChannelMessagesConfigured<'_>, Vec<Message>);
impl_exec_model_ext!(GetCurrentUser<'_>, CurrentUser);
impl_exec_model_ext!(GetCurrentUserGuildMember<'_>, Member);
impl_exec_model_ext!(GetEmojis<'_>, Vec<Emoji>);
impl_exec_model_ext!(GetGuild<'_>, Guild);
impl_exec_model_ext!(GetGuildRoles<'_>, Vec<Role>);
impl_exec_model_ext!(GetMember<'_>, Member);
impl_exec_model_ext!(GetMessage<'_>, Message);
impl_exec_model_ext!(GetUser<'_>, User);
impl_exec_model_ext!(GetUserApplicationInfo<'_>, Application);
impl_exec_model_ext!(UpdateMessage<'_>, Message);
impl_exec_model_ext!(UpdateResponse<'_>, Message);

/// Macro to simplify manual non-exhaustive `Debug` impl.
pub macro impl_debug_struct_fields($t:ty, $($field:ident),*) {
    impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct(stringify!($t))
            $(.field(stringify!($field), &self.$field))*
            .finish_non_exhaustive()
        }
    }
}

/// Create a slightly nicer, comma separated, list from a slice.
pub fn nice_list<T: Display>(list: &[T]) -> impl Display {
    let mut list = list.iter();
    let mut out = list.next().map(|s| format!("`{}`", s)).unwrap_or_default();

    for item in list {
        out = format!("{}", format_args!("{out}, `{item}`"));
    }

    out
}

/// Mega dum-dum escaping, may or may not work as expected.
pub fn escape_discord_chars(text: &str) -> Cow<'_, str> {
    let escape = &['|', '\\', '`', '<', '*', '_', '~'];

    if !text.contains(escape) {
        // At least we don't have to do much if all is well.
        return Cow::Borrowed(text);
    }

    let mut out = String::with_capacity(text.len());

    for ch in text.chars() {
        if escape.contains(&ch) {
            out.push('\\');
        }

        out.push(ch);
    }

    Cow::Owned(out)
}

/// Display reaction in discord emoji format.
/// Returns `Err(id)` *(id as string)* if emoji name is unavailable.
pub fn display_reaction_emoji(reaction: &ReactionType) -> Result<String, String> {
    match reaction {
        ReactionType::Custom {
            animated: true,
            id,
            name: Some(n),
        } => Ok(format!("<a:{n}:{id}>")),
        ReactionType::Custom {
            animated: false,
            id,
            name: Some(n),
        } => Ok(format!("<:{n}:{id}>")),
        ReactionType::Custom { id, name: None, .. } => Err(id.to_string()), // This should only happen if emoji was deleted from the guild, or something.
        ReactionType::Unicode { name } => Ok(name.to_string()),
    }
}

/// Format `obj` with a pretty json formatter with 4 space indent.
/// # Panics
/// This will panic if serialization failed or output is invalid utf-8.
pub fn pretty_nice_json<S: Serialize>(obj: S) -> String {
    let pretty = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(Vec::new(), pretty);
    obj.serialize(&mut ser).unwrap();
    String::from_utf8(ser.into_inner()).unwrap()
}

#[derive(Debug, PartialEq, Eq)]
pub struct Shenanigans<'a> {
    id: Option<Id<EmojiMarker>>,
    name: Option<&'a str>,
}

impl<'a> From<&'a ReactionType> for Shenanigans<'a> {
    fn from(other: &'a ReactionType) -> Self {
        match other {
            ReactionType::Custom { id, name, .. } => Self {
                id: Some(*id),
                name: name.as_deref(),
            },
            ReactionType::Unicode { name } => Self {
                id: None,
                name: Some(name),
            },
        }
    }
}
