use crate::config::ChannelAcceptanceParams;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tonic_lnd::lnrpc::ChannelAcceptRequest;

use crate::config::LndConfig;
use secp256k1::PublicKey;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic_lnd::lnrpc::{
    ChannelAcceptResponse, SendCustomMessageRequest, SubscribeCustomMessagesRequest,
};
use tonic_lnd::LightningClient;

use serde_json::{json, Value};

pub(crate) async fn create_client(cfg: LndConfig) -> LightningClient {
    tracing::info!("starting connection to lnd");
    tonic_lnd::connect(
        cfg.host,
        cfg.port as u32,
        cfg.cert_location,
        cfg.macaroon_location,
    )
    .await
    .expect("could not connect to LND")
    .lightning()
    .clone()
}

pub(crate) async fn start_channel_acceptor(
    lnd_client: LightningClient,
    whitelisted_channels: Vec<ChannelAcceptanceParams>,
) {
    tracing::info!("starting channel acceptor");

    // convert vec to hashmap for easy access
    let mut whitelist: HashMap<String, i8> = HashMap::new();
    whitelisted_channels.into_iter().for_each(|c| {
        whitelist
            .entry(c.pubkey)
            .and_modify(|conf| *conf = c.confs)
            .or_insert(c.confs);
    });

    let _ = tokio::join!(
        async {
            loop {
                start_listening(lnd_client.clone(), whitelist.clone()).await;
                tracing::debug!("restarting channel acceptor in 5s...");
                sleep(Duration::from_secs(5)).await;
            }
        },
        async {
            loop {
                start_listening_custommsg(lnd_client.clone(), whitelist.clone()).await;
                tracing::debug!("restarting custommsg in 5s...");
                sleep(Duration::from_secs(5)).await;
            }
        },
    );
}

async fn start_listening_custommsg(
    mut lnd_client: LightningClient,
    whitelist: HashMap<String, i8>,
) {
    let request: SubscribeCustomMessagesRequest = Default::default();

    let mut custommsg_stream = {
        match lnd_client.subscribe_custom_messages(request).await {
            Ok(response) => response.into_inner(),
            Err(..) => {
                // TODO: bubble the error
                tracing::error!("custommsg failed before receiving first message..");
                return;
            }
        }
    };

    tracing::info!("custommsg started");
    while let Some(custommsg) = custommsg_stream
        .message()
        .await
        .expect("Failed to receive custommsg")
    {
        // 'type' is a keyword in rust, so we need to use r#type
        if custommsg.r#type != 55443 {
            tracing::error!("custommsg: Unrecognized message type in incoming request");
            continue;
        }

        let msg_contents: Value = match serde_json::from_slice(&custommsg.data) {
            Ok(v) => v,
            Err(..) => {
                tracing::error!("custommsg: Non-JSON decodable data in incoming request");
                continue;
            }
        };

        if msg_contents.get("method") != Some(&json!("getzeroconfinfo")) {
            tracing::error!("custommsg: Unrecognized method in incoming request");
            continue;
        }

        let msg_id = match msg_contents.get("id") {
            Some(id) => id,
            None => {
                tracing::error!("custommsg: No id field in incoming request");
                continue;
            }
        };

        let node_pubkey = PublicKey::from_slice(&custommsg.peer)
            .expect("node_pubkey should be a PublicKey")
            .to_string();

        let reply_contents = json!({
               "id": msg_id,
               "result": {
                   "allows_your_zeroconf": whitelist.get(&node_pubkey) == Some(&0),
               }
        });

        tracing::debug!("custommsg: Replying affirmatively to {}", msg_id);

        let reply_data: Vec<u8> = match serde_json::to_vec(&reply_contents) {
            Ok(vec) => vec,
            Err(..) => {
                tracing::error!("custommsg: Could serialize affirmative reply");
                continue;
            }
        };

        // 'type' is a keyword in rust, so we need to use r#type
        let reply = SendCustomMessageRequest {
            peer: custommsg.peer,
            r#type: 55445,
            data: reply_data,
        };

        if lnd_client.send_custom_message(reply).await.is_err() {
            tracing::error!("custommsg: Could not send affirmative reply");
            continue;
        }
    }
}

async fn start_listening(mut lnd_client: LightningClient, whitelist: HashMap<String, i8>) {
    let (tx, rx) = mpsc::channel::<ChannelAcceptResponse>(1024);
    let receive_stream = ReceiverStream::new(rx);
    let channel_acceptor_conn = lnd_client.channel_acceptor(receive_stream).await;
    if channel_acceptor_conn.is_err() {
        tracing::error!("channel acceptor failed before receiving first message..");
        return;
    }
    let mut channel_acceptor = channel_acceptor_conn
        .expect("channel acceptor should not have an error")
        .into_inner();

    tracing::info!("channel acceptor started");
    while let Some(channel) = channel_acceptor
        .message()
        .await
        .expect("Failed to receive HTLCs")
    {
        let node_pubkey = PublicKey::from_slice(&channel.node_pubkey)
            .expect("node_pubkey should be a PublicKey")
            .to_string();

        tracing::debug!("received request to accept channel from {node_pubkey}...");

        let confs_required = match whitelist.get(&node_pubkey) {
            Some(c) => *c,
            None => 6, // TODO support testnet/regtest only requiring 3
        };

        match confs_required {
            i8::MIN..=-1 => {
                // anything negative is a deny
                deny(tx.clone(), channel).await;
            }
            0 => {
                // node was in our whitelist with 0 confs required, accept...
                tracing::debug!("accepting zero conf channel...");
                accept(tx.clone(), channel, true, 0).await;
                tracing::info!("zero conf channel accepted from {node_pubkey}");
            }
            _ => {
                // accept all other channels, just don't indicate it's zero conf
                // these could either be whitelisted with a specific conf or
                // not whitelisted at all
                tracing::debug!("accept channel with confs: {confs_required}");
                accept(tx.clone(), channel, false, confs_required as u32).await;
                tracing::info!("accepted non-zero-conf channels from {node_pubkey}");
            }
        }
    }
}

async fn accept(
    tx: Sender<ChannelAcceptResponse>,
    channel: ChannelAcceptRequest,
    zero_conf: bool,
    min_accept_depth: u32,
) {
    // Only the accept and id are important, ignore the rest
    tx.send(ChannelAcceptResponse {
        accept: true,
        pending_chan_id: channel.pending_chan_id,
        error: String::from(""),
        upfront_shutdown: String::from(""),
        csv_delay: 0,
        reserve_sat: 0,
        in_flight_max_msat: 0,
        max_htlc_count: 0,
        min_htlc_in: 0,
        min_accept_depth,
        zero_conf,
    })
    .await
    .unwrap()
}

async fn deny(tx: Sender<ChannelAcceptResponse>, channel: ChannelAcceptRequest) {
    // Only the accept and id are important, ignore the rest
    tx.send(ChannelAcceptResponse {
        accept: false,
        pending_chan_id: channel.pending_chan_id,
        error: String::from(""),
        upfront_shutdown: String::from(""),
        csv_delay: 0,
        reserve_sat: 0,
        in_flight_max_msat: 0,
        max_htlc_count: 0,
        min_htlc_in: 0,
        min_accept_depth: 0,
        zero_conf: false,
    })
    .await
    .unwrap()
}
