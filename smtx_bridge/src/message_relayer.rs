async pub fn send_to_swarm_b(swarm: &mut Swarm<YourBehaviour>, message: YourMessage) {
    // Determine the PeerId to send this message to.
    // How you do this will depend on your application's logic.
    let target_peer_id: PeerId = determine_target_peer_id_for_message(&message);

    // Use the behaviour object's API to send the message.
    // This is dependent on how you've set up your libp2p Behaviour.
    let behaviour = swarm.behaviour_mut();

    // Use your own protocol's method to send the message.
    // The exact method would depend on how you've implemented your libp2p Behaviour.
    if behaviour.your_custom_protocol().send_message(target_peer_id, message).is_err() {
        // Handle the error, e.g., by logging it or taking some corrective action.
    }
}
async pub fn send_to_swarm_a(swarm: &mut Swarm<YourBehaviour>, message: YourMessage) {
    // Determine the PeerId to send this message to.
    // How you do this will depend on your application's logic.
    let target_peer_id: PeerId = determine_target_peer_id_for_message(&message);

    // Use the behaviour object's API to send the message.
    // This is dependent on how you've set up your libp2p Behaviour.
    let behaviour = swarm.behaviour_mut();

    // Use your own protocol's method to send the message.
    // The exact method would depend on how you've implemented your libp2p Behaviour.
    if behaviour.your_custom_protocol().send_message(target_peer_id, message).is_err() {
        // Handle the error, e.g., by logging it or taking some corrective action.
    }
}
