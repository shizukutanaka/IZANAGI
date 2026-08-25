//! Wallet + dialogue + shop integration.
//!
//! `STRENGTHS_WEAKNESSES.md` (W8) flagged that these three modules were each
//! unit-tested in isolation but never proven to compose safely: a shopkeeper
//! conversation (`dialogue`) that branches on whether a purchase (`shop`,
//! backed by `wallet`) succeeds is exactly the kind of cross-module wiring a
//! real game performs, and it is where a subtle interaction — not a bug in any
//! one module — would surface.
//!
//! Three Socratic claims under test, mirroring `replay_integration.rs`'s
//! structure but for the economy/dialogue domain:
//!
//! 1. **Trace reproducibility**: replaying the same sequence of player choices
//!    against the same starting wallet/shop/dialogue state produces a
//!    bit-identical hash trace every time.
//! 2. **Divergence detection**: changing a single choice at tick K (e.g.
//!    buying instead of leaving) causes `check_trace` to report a divergence
//!    starting at or before exactly tick K.
//! 3. **Rollback fidelity**: `resimulate` from a mid-conversation `Clone`
//!    snapshot reproduces the unbroken run's final hash, and leaves the
//!    snapshot itself untouched.
//!
//! A fourth property closes the loop on the modules' own guarantees:
//! regardless of how the conversation branches, total currency across the
//! player's wallet and the shop's till is conserved, and the dialogue cursor
//! always reaches a stable, known state — the three modules never leave each
//! other in an inconsistent spot.

use izanagi_kit::dialogue::{Dialogue, DialogueNode};
use izanagi_kit::shop::Shop;
use izanagi_kit::wallet::Wallet;
use izanagi_kit::{
    check_trace, first_divergence, hash_state, record_trace, resimulate, DetHash, Fnv1a, SplitMix64,
};

const SEED: u64 = 0x5410_7000;
const TRIALS: usize = 150;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Coin {
    Gold,
}

impl DetHash for Coin {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u8(*self as u8);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Item {
    Sword,
    Potion,
}

impl DetHash for Item {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u8(*self as u8);
    }
}

/// Player action at the shopkeeper's greeting node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShopChoice {
    /// Attempt to buy the sword (100 gold).
    BuySword,
    /// Attempt to buy a potion (10 gold).
    BuyPotion,
    /// Attempt to sell the sword back (40 gold payout).
    SellSword,
    /// Leave without transacting.
    Leave,
}

/// Combined session state — the three modules under test, glued by the game
/// loop rather than by any coupling between the modules themselves. Dialogue
/// stays fully agnostic of wallet/shop; the harness decides which node to
/// jump to based on the *transaction's* outcome, exactly as a real game's
/// dialogue-driver would.
#[derive(Clone, Debug)]
struct ShopSession {
    wallet: Wallet<Coin>,
    shop: Shop<Item, Coin>,
    talk: Dialogue,
}

/// Node layout:
/// 0 = greeting (4 choices: buy sword / buy potion / sell sword / leave)
/// 1 = "here you go" (purchase or sale succeeded)
/// 2 = "no deal" (purchase or sale failed)
/// 3 = "safe travels" (left) — terminal, ends the conversation
fn greeting_nodes() -> Vec<DialogueNode> {
    vec![
        DialogueNode::new("What'll it be?")
            .with_choice("Buy the sword", 1) // targets are placeholders; `apply` overrides via goto()
            .with_choice("Buy a potion", 1)
            .with_choice("Sell my sword", 1)
            .with_choice("Just leaving", 3),
        DialogueNode::new("Here you go."),
        DialogueNode::new("No deal."),
        DialogueNode::new("Safe travels."),
    ]
}

impl ShopSession {
    fn new(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let mut wallet = Wallet::new();
        wallet.deposit(Coin::Gold, 50 + rng.below(120) as u64);

        let mut shop: Shop<Item, Coin> = Shop::new(Coin::Gold);
        shop.list(Item::Sword, 100, 40);
        shop.list(Item::Potion, 10, 3);
        shop.stock(200);

        let talk = Dialogue::new(greeting_nodes(), 0);

        ShopSession { wallet, shop, talk }
    }

    /// Apply one player action: perform the corresponding wallet/shop
    /// transaction (if any), navigate the dialogue to the node matching the
    /// outcome, then return to the greeting for another round (unless the
    /// player left). A no-op if the conversation isn't at the greeting
    /// (already ended, or mid-transition — never happens in this harness
    /// since `apply` always leaves the cursor at 0 or ended).
    fn apply(&mut self, action: &ShopChoice) {
        if self.talk.current_index() != Some(0) {
            return;
        }
        match action {
            ShopChoice::BuySword => {
                self.talk.choose(0);
                let ok = self.shop.buy(&mut self.wallet, Item::Sword);
                self.talk.goto(if ok { 1 } else { 2 });
            }
            ShopChoice::BuyPotion => {
                self.talk.choose(1);
                let ok = self.shop.buy(&mut self.wallet, Item::Potion);
                self.talk.goto(if ok { 1 } else { 2 });
            }
            ShopChoice::SellSword => {
                self.talk.choose(2);
                let ok = self.shop.sell(&mut self.wallet, Item::Sword);
                self.talk.goto(if ok { 1 } else { 2 });
            }
            ShopChoice::Leave => {
                self.talk.choose(3);
            }
        }
        // Return to the greeting for another round, unless the player left —
        // in which case the conversation is over for good.
        if self.talk.current_index() == Some(3) {
            self.talk.end();
        } else {
            self.talk.goto(0);
        }
    }

    fn total_currency(&self) -> u64 {
        self.wallet.balance(Coin::Gold) + self.shop.till_balance()
    }
}

impl DetHash for ShopSession {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.wallet.det_hash(hasher);
        self.shop.det_hash(hasher);
        self.talk.det_hash(hasher);
    }
}

/// Deterministically choose the next player action from the RNG stream.
fn pick_action(rng: &mut SplitMix64) -> ShopChoice {
    match rng.below(4) {
        0 => ShopChoice::BuySword,
        1 => ShopChoice::BuyPotion,
        2 => ShopChoice::SellSword,
        _ => ShopChoice::Leave,
    }
}

fn gen_actions(rng: &mut SplitMix64, n: usize) -> Vec<ShopChoice> {
    (0..n).map(|_| pick_action(rng)).collect()
}

fn step(session: &mut ShopSession, action: &ShopChoice) {
    session.apply(action);
}

// ── Claim 1: trace reproducibility ─────────────────────────────────────────

#[test]
fn test_identical_actions_yield_bit_identical_trace() {
    for trial in 0..TRIALS {
        let seed = SEED ^ (trial as u64);
        let mut gen_rng = SplitMix64::new(seed ^ 0xACE);
        let actions = gen_actions(&mut gen_rng, 20);

        let mut session_a = ShopSession::new(seed);
        let trace_a = record_trace(&mut session_a, &actions, step);

        let mut session_b = ShopSession::new(seed);
        let trace_b = record_trace(&mut session_b, &actions, step);
        assert_eq!(
            trace_a, trace_b,
            "trial {trial}: identical inputs must yield an identical trace"
        );

        let mut session_c = ShopSession::new(seed);
        assert_eq!(
            check_trace(&mut session_c, &actions, &trace_a, step),
            Ok(()),
            "trial {trial}: replay must reproduce the recorded trace"
        );
        assert_eq!(
            hash_state(&session_a),
            hash_state(&session_c),
            "trial {trial}: final states must agree too"
        );
    }
}

// ── Claim 2: divergence detection ──────────────────────────────────────────

#[test]
fn test_changed_action_diverges_at_or_before_the_changed_tick() {
    let mut gen_rng = SplitMix64::new(SEED ^ 0xD1F1);
    for trial in 0..TRIALS {
        let n = 8 + (trial % 12);
        let seed = SEED.wrapping_add(trial as u64 * 7919);
        let actions = gen_actions(&mut gen_rng, n);

        let mut baseline_session = ShopSession::new(seed);
        let baseline = record_trace(&mut baseline_session, &actions, step);

        // Flip the action at a random tick K to something different.
        let k = trial % n;
        let mut mutated = actions.clone();
        let alt = match mutated[k] {
            ShopChoice::BuySword => ShopChoice::Leave,
            ShopChoice::BuyPotion => ShopChoice::BuySword,
            ShopChoice::SellSword => ShopChoice::BuyPotion,
            ShopChoice::Leave => ShopChoice::SellSword,
        };
        mutated[k] = alt;

        let mut mutated_session = ShopSession::new(seed);
        let mutated_trace = record_trace(&mut mutated_session, &mutated, step);

        match first_divergence(&baseline, &mutated_trace) {
            Ok(()) => {
                // A flipped action can legitimately produce the same
                // post-state hash only if both actions were no-ops in this
                // wallet/shop state (e.g. both fail to afford, or leaving
                // early makes the rest of the tail irrelevant on both
                // sides) — assert the traces really are identical rather
                // than silently accepting any outcome.
                assert_eq!(
                    baseline, mutated_trace,
                    "trial {trial}: divergence undetected but traces differ"
                );
            }
            Err(div) => {
                assert!(
                    div.tick <= k,
                    "trial {trial}: divergence at tick {} must be at or before the changed tick {k}",
                    div.tick
                );
            }
        }
    }
}

// ── Claim 3: rollback fidelity ──────────────────────────────────────────────

#[test]
fn test_resimulate_from_snapshot_matches_unbroken_run() {
    let mut gen_rng = SplitMix64::new(SEED ^ 0x50B0);
    for trial in 0..TRIALS {
        let n = 10 + (trial % 10);
        let seed = SEED.wrapping_add(trial as u64 * 104_729);
        let actions = gen_actions(&mut gen_rng, n);

        let mut session = ShopSession::new(seed);
        let split = n / 2;
        for a in &actions[..split] {
            session.apply(a);
        }
        let snapshot = session.clone();
        let snapshot_hash_before = hash_state(&snapshot);

        for a in &actions[split..] {
            session.apply(a);
        }
        let unbroken_final = hash_state(&session);

        let resimulated = resimulate(&snapshot, &actions[split..], step);

        assert_eq!(
            hash_state(&resimulated),
            unbroken_final,
            "trial {trial}: resimulate from snapshot must match the unbroken run"
        );
        assert_eq!(
            hash_state(&snapshot),
            snapshot_hash_before,
            "trial {trial}: resimulate must not mutate the snapshot"
        );
    }
}

// ── Claim 4: cross-module invariants hold regardless of branch taken ───────

#[test]
fn test_currency_conserved_and_dialogue_reaches_stable_state() {
    let mut gen_rng = SplitMix64::new(SEED ^ 0xC0A1);
    for trial in 0..TRIALS {
        let seed = SEED.wrapping_add(trial as u64 * 15_485_863);
        let mut session = ShopSession::new(seed);
        let total_before = session.total_currency();

        let n = 5 + (trial % 15);
        let actions = gen_actions(&mut gen_rng, n);
        for a in &actions {
            session.apply(a);
            assert_eq!(
                session.total_currency(),
                total_before,
                "trial {trial}: every transaction must conserve wallet+till currency"
            );
        }

        // The dialogue must always be sitting on a valid, known node: either
        // back at the greeting (ready for another round) or ended (left).
        assert!(
            session.talk.current_index() == Some(0) || session.talk.is_ended(),
            "trial {trial}: dialogue must settle on greeting or ended, got {:?}",
            session.talk.current_index()
        );
    }
}
