use crate::network::error::NetworkError;
use crate::network::message::{NetworkMessage, TipInfo};
use crate::node::Node;

pub fn handle_message(
    node: &mut Node,
    message: NetworkMessage,
) -> Result<Option<NetworkMessage>, NetworkError> {
    match message {
        NetworkMessage::Ping { nonce } => Ok(Some(NetworkMessage::Pong { nonce })),
        NetworkMessage::Pong { .. } => Ok(None),
        NetworkMessage::GetTip => Ok(node
            .tip_height()
            .zip(node.tip_hash())
            .map(|(height, hash)| NetworkMessage::Tip(TipInfo { height, hash }))),
        NetworkMessage::Tip(_) => Ok(None),
        NetworkMessage::GetBlockByHeight { height } => Ok(node
            .ledger
            .block(&height)
            .cloned()
            .map(NetworkMessage::Block)),
        NetworkMessage::GetBlockByHash { hash } => Ok(node
            .cache
            .block_by_hash(&hash)
            .cloned()
            .map(NetworkMessage::Block)),
        NetworkMessage::Block(block) => {
            node.apply_block(block)?;
            Ok(None)
        }
        NetworkMessage::Transaction(transaction) => {
            node.submit_transaction(transaction)?;
            Ok(None)
        }
        NetworkMessage::GetPeers => Ok(Some(NetworkMessage::Peers(vec![]))),
        NetworkMessage::Peers(_) => Ok(None),
    }
}
