import { GossipSub } from "@chainsafe/libp2p-gossipsub";
import { SubscriptionChangeData } from "@libp2p/interface";
import PQueue from "p-queue";

const uniques = <T>(x: T[]): T[] => [...new Set(x)];

export const relayerPubsub = async ({
  pubsub,
  toujoursRelayer = [],
}: {
  pubsub: GossipSub;
  toujoursRelayer?: string[];
}) => {
  const requêtes: { [idPair: string]: string[] } = {};
  const queue = new PQueue({ concurrency: 1 });

  const gérerChangementAbonnement = ({
    detail,
  }: {
    detail: SubscriptionChangeData;
  }) => {
    // const sujetsDavant = uniques(Object.values(requêtes).flat());
    requêtes[detail.peerId.toString()] = detail.subscriptions
      .filter((s) => s.subscribe)
      .map((s) => s.topic);

    const sujetsMaintenant = uniques(Object.values(requêtes).flat());
    // On peut se (r)abonner à tout, parce que GossipSub filtre les sujets auxquels on est déjà abonnés
    sujetsMaintenant.forEach((s) => pubsub.subscribe(s));
    /* const désabonnements = sujetsDavant.filter(
      (s) => !sujetsMaintenant.includes(s) && !toujoursRelayer.includes(s),
    ); */
    // désabonnements.forEach((s) => pubsub.unsubscribe(s));

    // console.log({ sujetsMaintenant, désabonnements });
  };

  pubsub.addEventListener("subscription-change", gérerChangementAbonnement);

  // À faire : garder compte des requêtes par pair et désabonner lorsque plus nécessaire

  const fonctionStopAvant = pubsub.stop.bind(pubsub);
  const fonctionStopAprès = async () => {
    await queue.onIdle();
    await fonctionStopAvant();
  };
  pubsub.stop = fonctionStopAprès.bind(pubsub.stop);
};
