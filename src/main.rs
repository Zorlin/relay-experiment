use futures::stream::StreamExt;
// Removed duplicate import
use libp2p::{
    // Removed unused core::upgrade
    identity,
    noise, ping, relay, identify,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, Multiaddr, PeerId, SwarmBuilder, // Removed unused Transport
};
use std::error::Error;
use std::time::Duration;
// Removed unused tokio::runtime::Runtime import
use log::{info, error};

// Define the network behaviour combining multiple protocols
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "RelayEvent")]
struct RelayBehaviour {
    relay: relay::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

// Define the custom event type that the behaviour emits to the Swarm
// We aren't emitting custom events in this basic example, but this is where they would go.
#[derive(Debug)]
enum RelayEvent {
    Relay(relay::Event),
    Ping(ping::Event),
    Identify(identify::Event),
}

impl From<relay::Event> for RelayEvent {
    fn from(event: relay::Event) -> Self {
        RelayEvent::Relay(event)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::{
        ping, relay, identify,
        identity::Keypair,
        PeerId, Multiaddr,
        swarm::ConnectionId,
        core::Endpoint,
    };

    // Helper to create a dummy PeerId for testing
    fn dummy_peer_id() -> PeerId {
        PeerId::from(Keypair::generate_ed25519().public())
    }

    // Helper to create a dummy Multiaddr for testing
    fn dummy_multiaddr() -> Multiaddr {
        "/ip4/127.0.0.1/tcp/0".parse().unwrap()
    }

    #[test]
    fn test_from_relay_event() {
        let peer_id = dummy_peer_id();
        let relay_event = relay::Event::CircuitReq {
            src_peer_id: peer_id,
            src_relay_addr: dummy_multiaddr(),
            max_circuit_duration: Duration::from_secs(10),
            max_circuit_bytes: 1024,
            limited_relay: false, // Added field
            reservation: None, // Added field
        };
        let event: RelayEvent = relay_event.into();
        assert!(matches!(event, RelayEvent::Relay(relay::Event::CircuitReq { .. })));
    }

    #[test]
    fn test_from_ping_event() {
        let peer_id = dummy_peer_id();
        let ping_event = ping::Event {
            peer: peer_id,
            connection: ConnectionId::new_unchecked(0), // Use new_unchecked for simplicity in test
            result: Result::Ok(ping::Success::Ping { rtt: Duration::from_millis(10) }),
        };
        let event: RelayEvent = ping_event.into();
        assert!(matches!(event, RelayEvent::Ping(ping::Event { .. })));
    }

    #[test]
    fn test_from_identify_event() {
        let peer_id = dummy_peer_id();
        let public_key = Keypair::generate_ed25519().public();
        let identify_event = identify::Event::Received {
            peer_id,
            info: identify::Info {
                public_key,
                protocol_version: "test/1.0".to_string(),
                agent_version: "test-agent/0.1".to_string(),
                listen_addrs: vec![dummy_multiaddr()],
                protocols: vec!["/test/1".into()],
                observed_addr: dummy_multiaddr(),
            },
            connection: ConnectionId::new_unchecked(0), // Added field
        };
        let event: RelayEvent = identify_event.into();
        assert!(matches!(event, RelayEvent::Identify(identify::Event::Received { .. })));
    }

    // Example test for a SwarmEvent pattern match (demonstrates structure, not a real unit test)
    // This kind of test is more suited for integration tests where a real Swarm exists.
    #[test]
    fn test_swarm_event_match_structure() {
        let peer_id = dummy_peer_id();
        let dummy_endpoint = libp2p::core::ConnectedPoint::Listener {
             local_addr: dummy_multiaddr(),
             send_back_addr: dummy_multiaddr(),
        };
        let event = SwarmEvent::ConnectionEstablished {
            peer_id,
            connection_id: ConnectionId::new_unchecked(0),
            endpoint: dummy_endpoint,
            failed_addresses: &[],
            other_established: 0,
        };

        match event {
             SwarmEvent::ConnectionEstablished { peer_id: p, endpoint: e, .. } => {
                 assert_eq!(p, peer_id);
                 assert!(matches!(e, libp2p::core::ConnectedPoint::Listener{..}));
             },
             _ => panic!("Event did not match expected pattern"),
        }
    }
}

impl From<ping::Event> for RelayEvent {
    fn from(event: ping::Event) -> Self {
        RelayEvent::Ping(event)
    }
}

impl From<identify::Event> for RelayEvent {
    fn from(event: identify::Event) -> Self {
        RelayEvent::Identify(event)
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    info!("Starting Rust libp2p relay node...");

    // Create a random keypair for the node's identity
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer ID: {}", local_peer_id);

    // Transport construction is now handled by SwarmBuilder below.
    // We only need the configurations.

    // Create the Identify behaviour configuration
    // Note: The Identify protocol ID is now recommended to be just "/ipfs/id/1.0.0"
    // but we keep the custom one for now.
    let identify_config = identify::Config::new(
        "/libp2p-relay-rust/0.1.0".to_string(),
        local_key.public(),
    )
    .with_agent_version(format!("rust-libp2p-relay/{}", env!("CARGO_PKG_VERSION")));


    // Behaviour construction is now handled within the SwarmBuilder closure below.
    // The old 'let behaviour = ...' block has been removed.

    // Build the Swarm using the new builder pattern
    // Start with the identity, add the executor, configure the transport,
    // set timeouts/limits, add the behaviour, and build.
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new, // Use noise::Config::new directly here
            libp2p::yamux::Config::default, // Use yamux::Config::default directly here
        )?
        // Example: Set connection limits (optional)
        // .with_connection_limits(libp2p::connection_limits::ConnectionLimits::default())
        // Example: Set dial concurrency factor (optional)
        // .with_dial_concurrency_factor(std::num::NonZeroU8::new(8).unwrap())
        .with_behaviour(|key| {
            // The behaviour construction might need the keypair now,
            // although our current RelayBehaviour doesn't directly use it in its constructor.
            // We pass the peer_id derived earlier.
             // identify_config is captured by the closure now, no need to clone separately
             // as it wasn't moved previously.
             RelayBehaviour {
                // Use the peer_id derived from the key passed to the closure
                relay: relay::Behaviour::new(PeerId::from(key.public()), Default::default()),
                ping: ping::Behaviour::new(ping::Config::new()),
                identify: identify::Behaviour::new(identify_config), // Use identify_config directly
            }
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60))) // Example config
        .build();


    // Define listening addresses
    // Listen on all interfaces on TCP port 0, which asks the OS for a free port.
    let listen_addr_tcp = "/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?;
    swarm.listen_on(listen_addr_tcp)?;

    // Listen on QUIC as well (optional, requires enabling the 'quic' feature in Cargo.toml)
    // let listen_addr_quic = "/ip4/0.0.0.0/udp/0/quic-v1".parse::<Multiaddr>()?;
    // swarm.listen_on(listen_addr_quic)?;


    // Main event loop
    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("Listening on: {}", address);
                    }
                    SwarmEvent::Behaviour(RelayEvent::Identify(identify::Event::Received { peer_id, info })) => {
                        info!("Identified Peer: {} with agent version: {}", peer_id, info.agent_version);
                        // Add addresses observed by Identify to the Swarm's address book
                        for addr in info.listen_addrs {
                             swarm.add_external_address(addr);
                        }
                    }
                    SwarmEvent::Behaviour(RelayEvent::Ping(event)) => {
                         info!("Ping event: {:?}", event);
                    }
                    SwarmEvent::Behaviour(RelayEvent::Relay(event)) => {
                        info!("Relay event: {:?}", event);
                        // Handle specific relay events if needed
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        info!("Connection established with: {} on {:?}", peer_id, endpoint.get_remote_address());
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        info!("Connection closed with: {}. Reason: {:?}", peer_id, cause);
                    }
                    // Added `..` to ignore unmentioned fields like connection_id
                    SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => {
                        info!("Incoming connection from {} to {}", send_back_addr, local_addr);
                    }
                    // Added `..` to ignore unmentioned fields like connection_id
                    SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
                        error!("Incoming connection error from {} to {}: {}", send_back_addr, local_addr, error);
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        error!("Outgoing connection error to {:?}: {}", peer_id, error);
                    }
                    SwarmEvent::ListenerError { listener_id, error } => {
                        error!("Listener error for {:?}: {}", listener_id, error);
                    }
                     SwarmEvent::ListenerClosed { listener_id, reason, .. } => {
                        info!("Listener {:?} closed: {:?}", listener_id, reason);
                    }
                    SwarmEvent::Dialing { peer_id, connection_id } => {
                        info!("Dialing peer: {:?} (connection ID: {:?})", peer_id, connection_id);
                    }
                    _ => {
                        // Handle other swarm events as needed
                        // info!("Unhandled Swarm Event: {:?}", event);
                    }
                }
            }
        }
    }
}
