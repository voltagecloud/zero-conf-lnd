use crate::config::ChannelAcceptanceParams;
use std::collections::HashMap;

use crate::config::LndConfig;
use secp256k1::PublicKey;
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

    let (tx, rx) = tokio::sync::mpsc::channel::<ChannelAcceptResponse>(1024);
    let receive_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

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
        tracing::info!("received request to accept channel...");

        // check zero conf status
        if !channel.wants_zero_conf {}

        let node_pubkey = PublicKey::from_slice(&channel.node_pubkey)
            .expect("node_pubkey should be a PublicKey")
            .to_string();

        if let Some(whitelisted_channel) = whitelist.get(&node_pubkey) {
            // TODO parse the confirmation minimum and return the channel
            // acceptor response
        } else {
            // not in our whitelist, send default response
        }
    }
}
