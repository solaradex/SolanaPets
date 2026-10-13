/**
 * Solana program call stubs.
 *
 * Every care action will eventually submit a real transaction to the
 * pet-registry program (feed_pet, play_with_pet, clean_pet, rest_pet,
 * give_affection). Until the wallet adapter + IDL wiring lands, these
 * stubs log the EXACT instruction that would be sent and resolve after
 * a short delay so the UI flow (busy state → reaction → particles)
 * can be built and tested end to end.
 *
 * TODO(wallet):
 *  1. Add @solana/wallet-adapter + Anchor IDL for pet-registry.
 *  2. Replace each stub with program.methods.<ix>(petId).accounts({...}).rpc().
 *  3. Derive the Pet PDA: ["pet", petId as LE bytes] (see programs/pet-registry).
 *  4. Handle SimulationFailed / AnchorError and surface them in the HUD.
 */

const delay = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

let txCounter = 0;

async function stubCall(instruction: string, params: Record<string, unknown>): Promise<{ signature: string }> {
  const signature = `stub_tx_${++txCounter}_${Date.now().toString(36)}`;
  console.info(`[solana-stub] ${instruction}`, { ...params, signature });
  // Simulate network + confirmation latency.
  await delay(280 + Math.random() * 220);
  return { signature };
}

/** PDA seeds for the Pet account: ["pet", petId.toLEBytes()] */
function petPdaSeeds(petId: number): string {
  return `["pet", ${petId} as u32 LE bytes]`;
}

export const feedPet = (petId: number) =>
  stubCall('feed_pet', { pet: petPdaSeeds(petId), caregiver: '<wallet>' });

export const playWithPet = (petId: number) =>
  stubCall('play_with_pet', { pet: petPdaSeeds(petId), caregiver: '<wallet>' });

export const cleanPet = (petId: number) =>
  stubCall('clean_pet', { pet: petPdaSeeds(petId), caregiver: '<wallet>' });

export const restPet = (petId: number) =>
  stubCall('rest_pet', { pet: petPdaSeeds(petId), caregiver: '<wallet>' });

export const giveAffection = (petId: number) =>
  stubCall('give_affection', { pet: petPdaSeeds(petId), caregiver: '<wallet>' });
