//! Currency wallets — fungible balances for shops and economies.
//!
//! The item layer was well covered — [`inventory`](crate::inventory) stores
//! discrete items, [`equipment`](crate::equipment) wears them,
//! [`recipe`](crate::recipe) transforms them, [`affix`](crate::affix) enchants
//! them — but nothing tracked **fungible quantities of currency**: gold, gems,
//! tokens, dungeon scrip. That is the economy axis every shop, vendor, and
//! reward needs. [`Wallet<C>`] is that layer: a purse holding non-negative
//! balances of one or more currency kinds, with all-or-nothing spends and
//! atomic transfers between purses.
//!
//! ```
//! use izanagi_kit::wallet::Wallet;
//!
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! enum Coin { Gold, Gem }
//!
//! let mut purse: Wallet<Coin> = Wallet::new();
//! purse.deposit(Coin::Gold, 100);
//! purse.deposit(Coin::Gem, 3);
//!
//! // A sword costs 80 gold — spend is all-or-nothing.
//! assert!(purse.can_afford(Coin::Gold, 80));
//! assert!(purse.withdraw(Coin::Gold, 80));
//! assert_eq!(purse.balance(Coin::Gold), 20);
//! assert!(!purse.withdraw(Coin::Gold, 50)); // unaffordable → no change
//! assert_eq!(purse.balance(Coin::Gold), 20);
//!
//! // Pay a 2-gem toll into the shopkeeper's purse atomically.
//! let mut shop: Wallet<Coin> = Wallet::new();
//! assert!(purse.transfer(&mut shop, Coin::Gem, 2));
//! assert_eq!(purse.balance(Coin::Gem), 1);
//! assert_eq!(shop.balance(Coin::Gem), 2);
//! ```
//!
//! ## Design
//!
//! Balances are `u64` (non-negative; deposits saturate rather than overflow)
//! stored in a `BTreeMap<C, u64>` for deterministic iteration and hashing. A
//! balance that reaches `0` is pruned, so [`is_empty`](Wallet::is_empty) means
//! "no money of any kind" and iteration only ever yields positive balances.
//! [`Wallet`] implements [`DetHash`](crate::world_hash::DetHash), folding the
//! sorted `(currency, balance)` pairs into the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::BTreeMap;

/// A purse holding non-negative balances of one or more currency kinds `C`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wallet<C: Ord + Clone> {
    balances: BTreeMap<C, u64>,
}

impl<C: Ord + Clone> Wallet<C> {
    /// Create an empty wallet.
    pub fn new() -> Self {
        Wallet {
            balances: BTreeMap::new(),
        }
    }

    /// The balance of currency `c`, or `0` if none is held.
    pub fn balance(&self, c: C) -> u64 {
        self.balances.get(&c).copied().unwrap_or(0)
    }

    /// Add `amount` of currency `c` (saturating at `u64::MAX`).
    pub fn deposit(&mut self, c: C, amount: u64) {
        if amount == 0 {
            return;
        }
        let entry = self.balances.entry(c).or_insert(0);
        *entry = entry.saturating_add(amount);
    }

    /// `true` if the wallet holds at least `amount` of currency `c`.
    /// An `amount` of `0` is always affordable.
    pub fn can_afford(&self, c: C, amount: u64) -> bool {
        self.balance(c) >= amount
    }

    /// Remove exactly `amount` of currency `c` **only if affordable**,
    /// all-or-nothing. Returns `true` and deducts on success; returns `false`
    /// and leaves the wallet unchanged otherwise. A balance reaching `0` is
    /// pruned. Withdrawing `0` is a no-op that returns `true`.
    pub fn withdraw(&mut self, c: C, amount: u64) -> bool {
        if amount == 0 {
            return true;
        }
        match self.balances.get_mut(&c) {
            Some(bal) if *bal >= amount => {
                *bal -= amount;
                if *bal == 0 {
                    self.balances.remove(&c);
                }
                true
            }
            _ => false,
        }
    }

    /// Set the balance of `c` to an exact `amount`. A value of `0` removes the
    /// entry.
    pub fn set(&mut self, c: C, amount: u64) {
        if amount == 0 {
            self.balances.remove(&c);
        } else {
            self.balances.insert(c, amount);
        }
    }

    /// Remove currency `c` entirely, returning the balance it held (or `0`).
    pub fn remove(&mut self, c: C) -> u64 {
        self.balances.remove(&c).unwrap_or(0)
    }

    /// Move `amount` of currency `c` from this wallet into `other`, atomically:
    /// the transfer happens only if this wallet can afford it, and either both
    /// wallets change or neither does. Returns `true` on success. Transferring
    /// `0` is a no-op that returns `true`.
    pub fn transfer(&mut self, other: &mut Wallet<C>, c: C, amount: u64) -> bool {
        if !self.can_afford(c.clone(), amount) {
            return false;
        }
        // Withdraw cannot fail here (affordability already checked).
        let ok = self.withdraw(c.clone(), amount);
        debug_assert!(ok);
        other.deposit(c, amount);
        true
    }

    /// The number of distinct currency kinds with a positive balance.
    pub fn len(&self) -> usize {
        self.balances.len()
    }

    /// `true` if the wallet holds no currency of any kind.
    pub fn is_empty(&self) -> bool {
        self.balances.is_empty()
    }

    /// Remove all currency.
    pub fn clear(&mut self) {
        self.balances.clear();
    }

    /// Iterate over `(currency, balance)` pairs in ascending currency order.
    /// Only positive balances are yielded.
    pub fn iter(&self) -> impl Iterator<Item = (&C, u64)> {
        self.balances.iter().map(|(c, &b)| (c, b))
    }
}

impl<C: Ord + Clone + DetHash> DetHash for Wallet<C> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.balances.len() as u32);
        for (c, &b) in &self.balances {
            c.det_hash(hasher);
            hasher.write_u64(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let w: Wallet<u32> = Wallet::new();
        assert!(w.is_empty());
        assert_eq!(w.len(), 0);
        assert_eq!(w.balance(1), 0);
    }

    #[test]
    fn test_deposit_accumulates() {
        let mut w = Wallet::new();
        w.deposit(1u32, 50);
        w.deposit(1, 30);
        assert_eq!(w.balance(1), 80);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn test_deposit_zero_is_noop() {
        let mut w = Wallet::new();
        w.deposit(1u32, 0);
        assert!(w.is_empty(), "depositing 0 creates no entry");
    }

    #[test]
    fn test_deposit_saturates() {
        let mut w = Wallet::new();
        w.deposit(1u32, u64::MAX);
        w.deposit(1, 100);
        assert_eq!(w.balance(1), u64::MAX, "deposit saturates, no overflow");
    }

    #[test]
    fn test_withdraw_all_or_nothing() {
        let mut w = Wallet::new();
        w.deposit(1u32, 100);
        assert!(w.withdraw(1, 40));
        assert_eq!(w.balance(1), 60);
        assert!(!w.withdraw(1, 1000), "unaffordable withdraw fails");
        assert_eq!(w.balance(1), 60, "failed withdraw leaves balance unchanged");
    }

    #[test]
    fn test_withdraw_to_zero_prunes() {
        let mut w = Wallet::new();
        w.deposit(1u32, 50);
        assert!(w.withdraw(1, 50));
        assert!(w.is_empty(), "zero balance is pruned");
    }

    #[test]
    fn test_withdraw_zero_is_noop_true() {
        let mut w = Wallet::new();
        w.deposit(1u32, 10);
        assert!(w.withdraw(1, 0));
        assert_eq!(w.balance(1), 10);
    }

    #[test]
    fn test_can_afford() {
        let mut w = Wallet::new();
        w.deposit(1u32, 40);
        assert!(w.can_afford(1, 40));
        assert!(w.can_afford(1, 0));
        assert!(!w.can_afford(1, 41));
        assert!(w.can_afford(2, 0), "0 of an absent currency is affordable");
    }

    #[test]
    fn test_set_and_remove() {
        let mut w = Wallet::new();
        w.set(1u32, 99);
        assert_eq!(w.balance(1), 99);
        w.set(1, 0);
        assert!(w.is_empty(), "set to 0 removes");
        w.set(2, 5);
        assert_eq!(w.remove(2), 5);
        assert_eq!(w.remove(2), 0, "second remove returns 0");
    }

    #[test]
    fn test_transfer_atomic_success() {
        let mut a = Wallet::new();
        let mut b = Wallet::new();
        a.deposit(1u32, 100);
        assert!(a.transfer(&mut b, 1, 60));
        assert_eq!(a.balance(1), 40);
        assert_eq!(b.balance(1), 60);
    }

    #[test]
    fn test_transfer_failure_changes_nothing() {
        let mut a = Wallet::new();
        let mut b = Wallet::new();
        a.deposit(1u32, 30);
        b.deposit(1, 5);
        assert!(!a.transfer(&mut b, 1, 100), "unaffordable transfer fails");
        assert_eq!(a.balance(1), 30, "sender unchanged");
        assert_eq!(b.balance(1), 5, "receiver unchanged");
    }

    #[test]
    fn test_transfer_conserves_total() {
        let mut a = Wallet::new();
        let mut b = Wallet::new();
        a.deposit(1u32, 70);
        b.deposit(1, 30);
        a.transfer(&mut b, 1, 25);
        assert_eq!(a.balance(1) + b.balance(1), 100, "currency is conserved");
    }

    #[test]
    fn test_multi_currency() {
        let mut w = Wallet::new();
        w.deposit(1u32, 100); // gold
        w.deposit(2, 5); // gems
        assert_eq!(w.len(), 2);
        assert!(w.withdraw(1, 50));
        assert_eq!(w.balance(1), 50);
        assert_eq!(w.balance(2), 5, "withdrawing one currency leaves others");
    }

    #[test]
    fn test_iter_sorted_positive_only() {
        let mut w = Wallet::new();
        w.deposit(9u32, 1);
        w.deposit(2, 1);
        w.deposit(5, 1);
        let keys: Vec<u32> = w.iter().map(|(c, _)| *c).collect();
        assert_eq!(keys, vec![2, 5, 9], "iteration in ascending currency order");
        for (_, b) in w.iter() {
            assert!(b > 0, "only positive balances are yielded");
        }
    }

    #[test]
    fn test_clear() {
        let mut w = Wallet::new();
        w.deposit(1u32, 10);
        w.deposit(2, 20);
        w.clear();
        assert!(w.is_empty());
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a = Wallet::new();
        a.deposit(1u32, 100);
        a.deposit(2, 50);
        // Order of deposits must not affect the hash.
        let mut b = Wallet::new();
        b.deposit(2u32, 50);
        b.deposit(1, 100);
        assert_eq!(hash_state(&a), hash_state(&b), "order-independent hash");

        let mut c = Wallet::new();
        c.deposit(1u32, 100);
        c.deposit(2, 51); // one different balance
        assert_ne!(hash_state(&a), hash_state(&c), "different balance → different hash");
    }
}
