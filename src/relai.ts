import {
  FaultTolerance,
  PeerDiscovery,
  type Libp2p,
  type PrivateKey,
} from "@libp2p/interface";

import { noise } from "@chainsafe/libp2p-noise";
import { yamux } from "@chainsafe/libp2p-yamux";
import {
  circuitRelayServer,
  circuitRelayTransport,
} from "@libp2p/circuit-relay-v2";
import { identify, identifyPush } from "@libp2p/identify";
import { webSockets } from "@libp2p/websockets";
import { webRTC, webRTCDirect } from "@libp2p/webrtc";
import { webTransport } from "@libp2p/webtransport";
import { bootstrap } from "@libp2p/bootstrap";
import { pubsubPeerDiscovery } from "@libp2p/pubsub-peer-discovery";
import { autoNAT } from "@libp2p/autonat";
import { dcutr } from "@libp2p/dcutr";
import { type GossipSub, gossipsub } from "@chainsafe/libp2p-gossipsub";
import { tcp } from "@libp2p/tcp";
import { ping } from "@libp2p/ping";
import { keys } from "@libp2p/crypto";
import { createLibp2p } from "libp2p";
import { peerIdFromPrivateKey } from "@libp2p/peer-id";
import {
  fromString as uint8ArrayFromString,
  toString as uint8ArrayToString,
} from "uint8arrays";
import fs from "fs";
import { createPeerScoreParams } from "@chainsafe/libp2p-gossipsub/score";
import { FsDatastore } from "datastore-fs";
import { relayerPubsub } from "./utils.js";

const bootstrapList = process.env.RELAY_BOOTSTRAP_LIST?.split(",");
const pubsubPeerDiscoveryTopics =
  process.env.RELAY_PUBSUB_PEER_DISCOVERY_TOPICS?.split(",") || [];

export const obtClefPrivéeRelai = async () => {
  // Clef privée obtenue avec: console.log(server.peerId.privateKey.toString('hex'))
  // exemple : "08011240821cb6bc3d4547fcccb513e82e4d718089f8a166b23ffcd4a436754b6b0774cf07447d1693cd10ce11ef950d7517bad6e9472b41a927cd17fc3fb23f8c70cd99"
  const relayPrivKey = process.env.CLEF_PRIVEE_RELAI;

  if (relayPrivKey) {
    const encoded = uint8ArrayFromString(relayPrivKey, "base64");

    // L'identité de pair qui correspond à la clef privée ci-dessus
    // exemple : '12D3KooWAJjbRkp8FPF5MKgMU53aUTxWkqvDrs4zc1VMbwRwfsbE'
    const clefPrivée = keys.privateKeyFromRaw(encoded);
    return clefPrivée;
  }
  return undefined;
};

interface MyServiceComponents {
  privateKey: PrivateKey;
}

class ServiceClefPrivée {
  private privateKey: PrivateKey;

  constructor(components: MyServiceComponents) {
    this.privateKey = components.privateKey;
  }

  obtenirClef(): PrivateKey {
    return this.privateKey;
  }
}

export const créerNœud = async () => {
  const clefPrivée = await obtClefPrivéeRelai();

  const peerId = clefPrivée ? peerIdFromPrivateKey(clefPrivée) : undefined;

  const domaine = process.env.DOMAINE;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const peerDiscovery: ((components: any) => PeerDiscovery)[] = [
    pubsubPeerDiscovery({
      interval: 1000,
      topics: pubsubPeerDiscoveryTopics, // defaults to ['_peer-discovery._p2p._pubsub']
      listenOnly: false,
    }),
  ];
  if (bootstrapList)
    peerDiscovery.push(
      bootstrap({
        list: bootstrapList,
      }),
    );

  const dossierStockage = ".libp2p";
  const stockage = new FsDatastore(dossierStockage);

  const nœud = await createLibp2p({
    privateKey: clefPrivée,
    addresses: {
      listen: [
        "/ip4/0.0.0.0/tcp/12345/ws",
        "/webrtc",
        "/webtransport",
        "/p2p-circuit",
      ],
      announce: domaine
        ? [
            `/dns4/${domaine}/tcp/443/wss/p2p/${peerId?.toString()}`,
            `/dns4/${domaine}/tcp/80/ws/p2p/${peerId?.toString()}`,
          ]
        : undefined,
    },
    datastore: stockage,
    transports: [
      webSockets(),
      webRTC(),
      webRTCDirect(),
      webTransport(),
      tcp(),
      circuitRelayTransport(),
    ],
    transportManager: {
      faultTolerance: FaultTolerance.NO_FATAL,
    },
    connectionEncrypters: [noise()],
    streamMuxers: [yamux()],
    peerDiscovery,
    services: {
      ping: ping(),
      identify: identify({
        maxMessageSize: 1e6,
        maxInboundStreams: 50,
        maxOutboundStreams: 50,
      }),
      identifyPush: identifyPush({
        maxMessageSize: 1e6,
        maxInboundStreams: 50,
        maxOutboundStreams: 50,
      }),
      autoNAT: autoNAT(),
      dcutr: dcutr(),
      pubsub: gossipsub({
        // D: 0,
        // Dlo: 0,
        // Dhi: 0,
        // Dout: 0,
        // doPX: true,
        ignoreDuplicatePublishError: true,
        allowPublishToZeroTopicPeers: true,
        runOnLimitedConnection: true,
        canRelayMessage: true,
        scoreParams: createPeerScoreParams({
          IPColocationFactorWeight: 0,
        }),
      }),
      relay: circuitRelayServer({
        reservations: {
          maxReservations: Infinity,
          applyDefaultLimit: false,
        },
      }),
      obtClefPrivée: (components: MyServiceComponents) =>
        new ServiceClefPrivée(components),
    },
  });

  await relayerPubsub({
    pubsub: nœud.services.pubsub as GossipSub,
    toujoursRelayer: ["réseau-constellation"],
  });

  nœud.services.pubsub.subscribe("réseau-constellation");

  if (!peerId) {
    const clefPrivéeGénérée = nœud.services.obtClefPrivée.obtenirClef();
    const clefTexte = uint8ArrayToString(clefPrivéeGénérée.raw, "base64");
    fs.appendFileSync(".env", `\nCLEF_PRIVEE_RELAI=${clefTexte}`);
  }
  nœud.addEventListener("peer:discovery", (x) => {
    console.log(
      "Découvert : ",
      x.detail.id.toString(),
      x.detail.multiaddrs.map((a) => a.toString()),
    );
  });
  nœud.addEventListener("peer:connect", () => {
    console.log("Pairs: ", nœud.getPeers());
  });
  nœud.addEventListener("peer:disconnect", () => {
    console.log("Pairs: ", nœud.getPeers());
  });
  nœud.services.relay.addEventListener("relay:reservation", (x) => {
    console.log("Réservation ", x.detail.addr.toString());
  });
  nœud.services.relay.addEventListener("relay:advert:success", () => {
    console.log("relay:advert:success");
  });
  nœud.services.relay.addEventListener("relay:advert:error", () => {
    console.log("relay:advert:error");
  });
  return nœud;
};

export const obtAdressesNœud = (nœud: Libp2p): string[] => {
  console.log(`Nœud lancé avec id : ${nœud.peerId.toString()}`);
  console.log(
    "Le nœud écoute sur : ",
    nœud.getMultiaddrs().map((ma) => ma.toString()),
  );
  return nœud.getMultiaddrs().map((ma) => ma.toString());
  // génère une adresse de manière déterministe : /ip4/127.0.0.1/tcp/12345/ws/p2p/12D3KooWAJjbRkp8FPF5MKgMU53aUTxWkqvDrs4zc1VMbwRwfsbE
};
