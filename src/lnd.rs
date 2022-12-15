use crate::config::ChannelAcceptanceParams;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tonic_lnd::lnrpc::ChannelAcceptRequest;

use crate::config::LndConfig;
use secp256k1::PublicKey;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic_lnd::lnrpc::ChannelAcceptResponse;
use tonic_lnd::LightningClient;

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
    mut lnd_client: LightningClient,
    whitelisted_channels: Vec<ChannelAcceptanceParams>,
) {
    tracing::info!("starting channel acceptor");

    // convert vec to hashmap for easy access
    let mut whitelist: HashMap<String, u8> = HashMap::new();
    whitelisted_channels.into_iter().for_each(|c| {
        whitelist
            .entry(c.pubkey)
            .and_modify(|conf| *conf = c.confs)
            .or_insert(c.confs);
    });

    let (tx, rx) = mpsc::channel::<ChannelAcceptResponse>(1024);
    let receive_stream = ReceiverStream::new(rx);

    let mut channel_acceptor = lnd_client
        .channel_acceptor(receive_stream)
        .await
        .expect("could not start channel acceptor")
        .into_inner();

    tracing::info!("channel acceptor started");

    while let Some(channel) = channel_acceptor
        .message()
        .await
        .expect("Failed to receive HTLCs")
    {
        tracing::debug!("received request to accept channel...");

        // check zero conf status
        if !channel.wants_zero_conf {
            // TODO check the conf amount if less than normal
            tracing::debug!("accepting normal channel...");
            accept(tx.clone(), channel, true, 0).await;
            tracing::info!("normal channel accepted");
            continue;
        }

        let node_pubkey = PublicKey::from_slice(&channel.node_pubkey)
            .expect("node_pubkey should be a PublicKey")
            .to_string();

        if let Some(whitelisted_channel_confs) = whitelist.get(&node_pubkey) {
            // parse the confirmation minimum and return the channel acceptor response
            tracing::debug!("accepting zero conf channel...");
            accept(tx.clone(), channel, true, *whitelisted_channel_confs as u32).await;
            tracing::info!("zero conf channel accepted");
        } else {
            // not in our whitelist, send failure response
            tracing::debug!("denying zero conf channel...");
            deny(tx.clone(), channel).await;
            tracing::info!("zero conf channel denied");
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
        zero_conf: true,
    })
    .await
    .unwrap()
}
