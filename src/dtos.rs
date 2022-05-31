#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ServerMessage {
    NewMessage {
        id: u64,
        reply_to: Option<u64>,
        text: String,
        channel: String,
        sender_name: String,
    },
    UpdateUserCount(u32),
    TilePlaced(u32, u8),
    TilesPlaced(Vec<(u32, u8)>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RPCSendMessageInput {
    pub(crate) text: String,
    pub(crate) reply_to: Option<u64>,
    pub(crate) channel: String,
    pub(crate) sender_name: String,
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
