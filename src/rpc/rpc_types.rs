use std::{fmt::{Display, Debug}, ops::Deref};

use crate::{game::SetTileError, messages::chat_manager::ChatError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum RPCServerMessage {
    NewMessage {
        id: u64,
        reply_to: Option<u64>,
        text: String,
        channel: String,
        sender_name: String,
    },
    UpdateUserCount(u32),
    TilePlaced(u32, u8),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RPCSendMessageInput {
    pub text: String,
    pub reply_to: Option<u64>,
    pub channel: String,
    pub sender_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum RPCClientMessage {
    SendMessage(RPCSendMessageInput),
    PlaceTile(PlaceTileInput),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PlaceTileInput {
    pub idx: u32,
    pub tile: u8,
}

impl From<SetTileError> for RPCSetTileError {
    fn from(e: SetTileError) -> Self {
        Self(e)
    }
}
impl Deref for RPCSetTileError {
    type Target = SetTileError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct RPCSetTileError(SetTileError);
impl serde::Serialize for RPCSetTileError {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            SetTileError::OutOfBounds => s.serialize_str("OutOfBounds"),
            SetTileError::InvalidColor => s.serialize_str("InvalidColor"),
            SetTileError::RateLimited => s.serialize_str("RateLimited"),
        }
    }
}

impl Debug for RPCSetTileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            SetTileError::OutOfBounds => write!(f, "OutOfBounds"),
            SetTileError::InvalidColor => write!(f, "InvalidColor"),
            SetTileError::RateLimited => write!(f, "RateLimited"),
        }
    }
}
impl Display for RPCSetTileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            SetTileError::OutOfBounds => write!(f, "OutOfBounds"),
            SetTileError::InvalidColor => write!(f, "InvalidColor"),
            SetTileError::RateLimited => write!(f, "RateLimited"),
        }
    }
}
impl std::error::Error for RPCSetTileError {}

pub struct RPCSendMessageError(ChatError);
impl From<ChatError> for RPCSendMessageError {
    fn from(e: ChatError) -> Self {
        Self(e)
    }
}
impl Deref for RPCSendMessageError {
    type Target = ChatError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl serde::Serialize for RPCSendMessageError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            ChatError::RateLimited => serializer.serialize_str("RateLimited"),
            ChatError::InvalidMessageLength => serializer.serialize_str("InvalidMessageLength"),
        }
    }
}

impl Debug for RPCSendMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            ChatError::RateLimited => write!(f, "RateLimited"),
            ChatError::InvalidMessageLength => write!(f, "InvalidMessageLength"),
        }
    }
}
impl Display for RPCSendMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            ChatError::RateLimited => write!(f, "RateLimited"),
            ChatError::InvalidMessageLength => write!(f, "InvalidMessageLength"),
        }
    }
}

impl std::error::Error for RPCSendMessageError {}
