use libp2p::swarm::{NetworkBehaviour, ConnectionHandler};
use log::{info, warn, error};
use std::collections::VecDeque;
use prost::Message as ProstMessage; // Import for protobuf serialization

// Include the generated protobuf code directly within this module
mod webrtc_signaling_proto {
    include!(concat!(env!("OUT_DIR"), "/webrtc_signaling.rs"));
}
use self::webrtc_signaling_proto::Message as SignalingMessage;
use self::webrtc_signaling_proto::message::Type as MessageType;

// CORRECTED Imports (Round 13 - Following docs)
use libp2p::{
    core::{upgrade, transport::PortUse, Endpoint},
    swarm::ConnectionId, // Use swarm::ConnectionId
    futures::prelude::*,
    swarm::{
        handler::{
            ConnectionEvent, ConnectionHandlerEvent,
            DialUpgradeError as HandlerDialUpgradeError, FullyNegotiatedInbound,
            FullyNegotiatedOutbound, ListenUpgradeError,
        },
        Stream, StreamUpgradeError,
        // Remove SubstreamError, KeepAlive, ConnectionHandlerUpgrErr
        SubstreamProtocol, ConnectionDenied, FromSwarm,
        ToSwarm,
    },
    Multiaddr, PeerId,
};
use std::{
    io,
    task::{Context, Poll},
};
use void::Void;
use futures::{SinkExt, StreamExt}; // For stream handling

// WebRTC signaling protocol identifier
const PROTOCOL_NAME: &'static str = "/webrtc-signaling/0.0.1";

// Modify Event enum to include ICE candidates
#[derive(Debug)]
pub enum Event {
    ReceivedSdpOffer { peer: PeerId, offer: String },
    ReceivedSdpAnswer { peer: PeerId, answer: String },
    ReceivedIceCandidate { peer: PeerId, candidate: String },
    SignalingError { peer: PeerId, error: SignalingError },
}

#[derive(Debug, thiserror::Error)]
pub enum SignalingError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Upgrade error: {0}")]
    UpgradeError(#[from] StreamUpgradeError<Void>),
    #[error("Invalid message format: {0}")]
    FormatError(String),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

pub struct Behaviour {
    events: VecDeque<Event>,
}

impl Behaviour {
    pub fn new() -> Self {
        Self { events: VecDeque::new() }
    }

    // Send an SDP offer to the given peer
    pub fn send_offer(&mut self, peer: PeerId, offer: String) {
        info!("Queuing SDP offer to send to {}", peer);
        self.events.push_back(Event::ReceivedSdpOffer { peer, offer });
        // In a full implementation, we'd queue this to be sent via NotifyHandler
    }
    
    // Send an SDP answer to the given peer
    pub fn send_answer(&mut self, peer: PeerId, answer: String) {
        info!("Queuing SDP answer to send to {}", peer);
        self.events.push_back(Event::ReceivedSdpAnswer { peer, answer });
        // In a full implementation, we'd queue this to be sent via NotifyHandler
    }
    
    // Send an ICE candidate to the given peer
    pub fn send_ice_candidate(&mut self, peer: PeerId, candidate: String) {
        info!("Queuing ICE candidate to send to {}", peer);
        self.events.push_back(Event::ReceivedIceCandidate { peer, candidate });
        // In a full implementation, we'd queue this to be sent via NotifyHandler
    }
}

// Extended handler out event to include ICE candidates
#[derive(Debug)]
pub enum HandlerOutEvent {
    ReceivedOffer(String),
    ReceivedAnswer(String),
    ReceivedIceCandidate(String),
}

#[derive(Debug, Clone)]
pub struct SignalingConfig;

impl upgrade::UpgradeInfo for SignalingConfig {
    type Info = &'static str;
    type InfoIter = std::option::IntoIter<Self::Info>;
    fn protocol_info(&self) -> Self::InfoIter {
        Some(PROTOCOL_NAME).into_iter()
    }
}

impl upgrade::InboundUpgrade<Stream> for SignalingConfig {
    type Output = Stream;
    type Error = Void;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;
    fn upgrade_inbound(self, stream: Stream, _info: Self::Info) -> Self::Future {
        future::ready(Ok(stream))
    }
}

impl upgrade::OutboundUpgrade<Stream> for SignalingConfig {
    type Output = Stream;
    type Error = Void;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;
    fn upgrade_outbound(self, stream: Stream, _info: Self::Info) -> Self::Future {
        future::ready(Ok(stream))
    }
}

#[derive(Debug)]
pub struct Handler {
    events: VecDeque<ConnectionHandlerEvent<SignalingConfig, (), HandlerOutEvent>>,
    outbound_messages: VecDeque<SignalingMessage>,
    outbound_substream: Option<SubstreamState>,
    inbound_substream: Option<SubstreamState>,
    keep_alive: bool, // Changed from KeepAlive enum to bool
}

#[derive(Debug)]
enum SubstreamState {
    Negotiating,
    Idle(Stream),
    Reading,
    Writing,
    Closing,
    Error,
}

impl Handler {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
            outbound_messages: VecDeque::new(),
            inbound_substream: None,
            outbound_substream: None,
            keep_alive: true, // Changed from KeepAlive::Yes to true
        }
    }
    
    // Process an incoming message
    fn process_message(&mut self, message: SignalingMessage) -> Result<(), SignalingError> {
        match message.r#type() {
            MessageType::SdpOffer => {
                if let Some(data) = message.data {
                    info!("Received SDP offer: {}", data);
                    let event = HandlerOutEvent::ReceivedOffer(data);
                    self.events.push_back(ConnectionHandlerEvent::NotifyBehaviour(event));
                    Ok(())
                } else {
                    Err(SignalingError::FormatError("SDP offer missing data".into()))
                }
            },
            MessageType::SdpAnswer => {
                if let Some(data) = message.data {
                    info!("Received SDP answer: {}", data);
                    let event = HandlerOutEvent::ReceivedAnswer(data);
                    self.events.push_back(ConnectionHandlerEvent::NotifyBehaviour(event));
                    Ok(())
                } else {
                    Err(SignalingError::FormatError("SDP answer missing data".into()))
                }
            },
            MessageType::IceCandidate => {
                if let Some(data) = message.data {
                    info!("Received ICE candidate: {}", data);
                    let event = HandlerOutEvent::ReceivedIceCandidate(data);
                    self.events.push_back(ConnectionHandlerEvent::NotifyBehaviour(event));
                    Ok(())
                } else {
                    Err(SignalingError::FormatError("ICE candidate missing data".into()))
                }
            },
        }
    }
    
    // Asynchronously read and process messages from the stream
    async fn read_messages(mut stream: Stream) -> Result<(), SignalingError> {
        // Read the length-prefixed protobuf message
        let mut buf = vec![0u8; 1024]; // Initial buffer size
        
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => {
                    // End of stream
                    return Ok(());
                },
                Ok(n) => {
                    // Try to decode the message
                    match SignalingMessage::decode(&buf[..n]) {
                        Ok(message) => {
                            info!("Decoded message: {:?}", message);
                            // In practice, you'd process the message here,
                            // but since this is an async method and Handler methods aren't,
                            // we'd need to send it through a channel
                        },
                        Err(e) => {
                            return Err(SignalingError::FormatError(format!("Failed to decode message: {}", e)));
                        }
                    }
                },
                Err(e) => {
                    return Err(SignalingError::Io(e));
                }
            }
        }
    }
    
    // Asynchronously write a message to the stream
    async fn write_message(mut stream: Stream, message: SignalingMessage) -> Result<(), SignalingError> {
        // Encode the message
        let mut buf = Vec::new();
        message.encode(&mut buf)
            .map_err(|e| SignalingError::FormatError(format!("Failed to encode message: {}", e)))?;
            
        // Write the message to the stream
        stream.write_all(&buf).await
            .map_err(|e| SignalingError::Io(e))?;
            
        Ok(())
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = SignalingMessage;
    type ToBehaviour = HandlerOutEvent;
    type InboundProtocol = SignalingConfig;
    type OutboundProtocol = SignalingConfig;
    type OutboundOpenInfo = ();
    type InboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(SignalingConfig, ())
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::ToBehaviour>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        if !self.keep_alive {
            // Do nothing; keep_alive=false will eventually cause the connection to close
        }

        Poll::Pending
    }

    fn on_behaviour_event(&mut self, message: Self::FromBehaviour) {
        self.outbound_messages.push_back(message);
        if self.outbound_substream.is_none() {
            self.events.push_back(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(SignalingConfig, ()),
            });
            self.outbound_substream = Some(SubstreamState::Negotiating);
        }
    }

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(fully_negotiated) => {
                info!("Signaling protocol negotiated (inbound)");
                self.inbound_substream = Some(SubstreamState::Idle(fully_negotiated.protocol));
            }
            ConnectionEvent::FullyNegotiatedOutbound(fully_negotiated) => {
                info!("Signaling protocol negotiated (outbound)");
                self.outbound_substream = Some(SubstreamState::Idle(fully_negotiated.protocol));
            }
            ConnectionEvent::DialUpgradeError(HandlerDialUpgradeError { error, .. }) => {
                warn!("Signaling dial upgrade error: {:?}", error);
                self.outbound_substream = Some(SubstreamState::Error);
                self.keep_alive = false; // Changed from KeepAlive::No
            }
            ConnectionEvent::ListenUpgradeError(ListenUpgradeError { error, .. }) => {
                warn!("Signaling listen upgrade error: {:?}", error);
            }
            ConnectionEvent::AddressChange(_) => {}
            // Add wildcard pattern for non-exhaustive enum
            _ => {}
        }
    }

    fn connection_keep_alive(&self) -> bool { // Changed return type from KeepAlive to bool
        self.keep_alive
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = Handler;
    type ToSwarm = Event;

    fn handle_pending_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        Ok(())
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(Handler::new())
    }

     fn handle_pending_outbound_connection(
         &mut self,
         _connection_id: ConnectionId,
         _maybe_peer: Option<PeerId>,
         _addresses: &[Multiaddr],
         _effective_role: Endpoint,
     ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
         Ok(Vec::new())
     }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &Multiaddr,
        _role_override: Endpoint,
        _port_use: PortUse,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(Handler::new())
    }

    fn on_swarm_event(&mut self, _event: FromSwarm) {}

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _connection_id: ConnectionId,
        event: HandlerOutEvent,
    ) {
        match event {
            HandlerOutEvent::ReceivedOffer(offer) => {
                info!("Received SDP Offer from {}", peer_id);
                self.events.push_back(Event::ReceivedSdpOffer { peer: peer_id, offer });
            }
            HandlerOutEvent::ReceivedAnswer(answer) => {
                info!("Received SDP Answer from {}", peer_id);
                self.events.push_back(Event::ReceivedSdpAnswer { peer: peer_id, answer });
            }
            HandlerOutEvent::ReceivedIceCandidate(candidate) => {
                info!("Received ICE Candidate from {}", peer_id);
                self.events.push_back(Event::ReceivedIceCandidate { peer: peer_id, candidate });
            }
        }
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, SignalingMessage>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(ToSwarm::GenerateEvent(event));
        }

        Poll::Pending
    }
}