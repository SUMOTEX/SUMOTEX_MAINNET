use async_trait::async_trait;
use libp2p::{
    swarm::{NetworkBehaviour,
         ConnectionId, 
         IntoProtocolsHandler,
         NetworkBehaviourEventProcess,
         NetworkBehaviourAction, PollParameters, ProtocolsHandler},
    PeerId,

    Multiaddr
};
use std::collections::VecDeque;
use futures::prelude::*;
use std::task::{Context, Poll};
use std::marker::PhantomData;
use std::io;
struct MyProtocolsHandler;

use async_trait::async_trait;
use std::collections::VecDeque;
use futures::prelude::*;
use std::task::{Context, Poll};
use std::marker::PhantomData;

struct MyProtocolsHandler;

impl ProtocolsHandler for MyProtocolsHandler {
    type InEvent = ();
    type OutEvent = ();
    type Error = io::Error;
    type InboundProtocol = ();
    type OutboundProtocol = ();
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        todo!()
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as InboundUpgrade>::Output,
        info: Self::InboundOpenInfo,
    ) {
        todo!()
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: <Self::OutboundProtocol as OutboundUpgrade>::Output,
        info: Self::OutboundOpenInfo,
    ) {
        todo!()
    }

    fn inject_event(&mut self, event: Self::InEvent) {
        todo!()
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        error: ProtocolsHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgrade>::Error>
    ) {
        todo!()
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        todo!()
    }
    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ProtocolsHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::OutEvent, Self::Error>> {
        // Your code here
        Poll::Pending
    }
    
    
}

pub struct SumotexProtocol {
    // Queue of messages to send.
    send_queue: VecDeque<(PeerId, String)>,
}

impl SumotexProtocol {
    pub fn new() -> Self {
        Self {
            send_queue: VecDeque::new(),
        }
    }

    pub fn send_message(&mut self, target: PeerId, message: String) {
        self.send_queue.push_back((target, message));
    }
}

impl NetworkBehaviour for SumotexProtocol {

    type ProtocolsHandler = MyProtocolsHandler;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        MyProtocolsHandler
    }

    fn inject_event(
        &mut self, 
        peer_id: PeerId, 
        connection: ConnectionId, 
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent
    ) {
        // Handle or ignore the events from your ProtocolsHandler here.
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _peer_id: &PeerId) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId) {}

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<NetworkBehaviourAction<(), Self::ProtocolsHandler>> {
        // Your code here
        Poll::Pending
    }
}
