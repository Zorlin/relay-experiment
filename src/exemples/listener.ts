import { noise } from "@chainsafe/libp2p-noise";
import { yamux } from "@chainsafe/libp2p-yamux";
import { circuitRelayTransport } from "@libp2p/circuit-relay-v2";
import { identify } from "@libp2p/identify";
import { multiaddr } from "@multiformats/multiaddr";
import { webSockets } from "@libp2p/websockets";
import { createLibp2p } from "libp2p";

const relayAddr = process.argv[2];
if (!relayAddr) {
  throw new Error("the relay address needs to be specified as a parameter");
}

const node = await createLibp2p({
  addresses: {
    listen: ["/p2p-circuit"],
  },
  transports: [webSockets(), circuitRelayTransport()],
  connectionEncrypters: [noise()],
  streamMuxers: [yamux()],
  services: {
    identify: identify(),
  },
});

console.log(`Node started with id ${node.peerId.toString()}`);

const conn = await node.dial(multiaddr(relayAddr));

console.log(`Connected to the relay ${conn.remotePeer.toString()}`);

// Wait for connection and relay to be bind for the example purpose
node.addEventListener("self:peer:update", (_evt) => {
  // Updated self multiaddrs?
  console.log(
    `Advertising with a relay address of ${node.getMultiaddrs()[0].toString()}`,
  );
});

/*
As you can see in the code, we need to provide the relay address, relayAddr, as a process argument. This node will dial the provided relay address and automatically bind to it.

You should now run the following to start the node running Auto Relay:

node listener.js /ip4/192.168.1.120/tcp/61592/ws/p2p/QmWDn2LY8nannvSWJzruUYoLZ4vV83vfCBwd8DipvdgQc3

This should print out something similar to the following:

Node started with id QmerrWofKF358JE6gv3z74cEAyL7z1KqhuUoVfGEynqjRm
Connected to the HOP relay QmWDn2LY8nannvSWJzruUYoLZ4vV83vfCBwd8DipvdgQc3
Advertising with a relay address of /ip4/192.168.1.120/tcp/61592/ws/p2p/QmWDn2LY8nannvSWJzruUYoLZ4vV83vfCBwd8DipvdgQc3/p2p-circuit/p2p/QmerrWofKF358JE6gv3z74cEAyL7z1KqhuUoVfGEynqjRm

Per the address, it is possible to verify that the auto relay node is listening on the circuit relay node address.

Instead of dialing this relay manually, you could set up this node with the Bootstrap module and provide it in the bootstrap list. Moreover, you can use other peer-discovery modules to discover peers in the network and the node will automatically bind to the relays that support HOP until reaching the maximum number of listeners.
*/
