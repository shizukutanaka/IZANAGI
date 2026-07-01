//! Shop pricing — buy/sell transactions against a single-currency price list.
//!
//! [`wallet`](crate::wallet) gave every purse atomic transfers, but nothing
//! tied an item to a **price**: how much a player pays to buy it, and how much
//! a vendor pays to buy it back. That is the missing shop layer — a listing of
//! `(item → buy_price, sell_price)` backed by the shop's own till (a
//! [`Wallet`]), so both directions of trade are the same all-or-nothing
//! transfer primitive `Wallet` already provides.
//!
//! ```
//! use izanagi_kit::shop::Shop;
//! use izanagi_kit::wallet::Wallet;
//!
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! enum Coin { Gold }
//!
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! enum Item { Sword, Potion }
//!
//! let mut shop: Shop<Item, Coin> = Shop::new(Coin::Gold);
//! shop.list(Item::Sword, 100, 40); // buy for 100, sell back for 40
//! shop.stock(500); // the shop's till starts with 500 gold to buy items back
//!
//! let mut player: Wallet<Coin> = Wallet::new();
//! player.deposit(Coin::Gold, 150);
//!
//! assert!(shop.buy(&mut player, Item::Sword));
//! assert_eq!(player.balance(Coin::Gold), 50);
//! assert_eq!(shop.till_balance(), 600);
//!
//! assert!(shop.sell(&mut player, Item::Sword));
//! assert_eq!(player.balance(Coin::Gold), 90);
//! assert_eq!(shop.till_balance(), 560);
//! ```
//!
//! ## Design
//!
//! - Listings live in a `BTreeMap<K, Listing>` for canonical iteration and
//!   hashing (no `HashMap` non-determinism).
//! - `buy`/`sell` are all-or-nothing: unlisted item, unaffordable buyer, or an
//!   under-funded till leaves both wallets untouched and returns `false`.
//! - The till is a plain [`Wallet`], so [`stock`](Shop::stock) /
//!   [`drain_till`](Shop::drain_till) reuse its existing deposit/withdraw
//!   semantics rather than duplicating them.
//! - No RNG, no float — every quantity is an integer, so shop transactions are
//!   replay-safe by construction.

use crate::wallet::Wallet;
use crate::world_hash::{DetHash, Fnv1a};
use std::collections::BTreeMap;

/// The price of one item: what a buyer pays, and what the shop pays back on a
/// sale. The two are independent — a shop is not required to buy back at a
/// loss-free markup, or to buy an item back at all (set `sell_price` to `0`
/// and [`Shop::sell`] always fails for it, since a `0` transfer is a no-op
/// that still requires the item be listed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listing {
    pub buy_price: u64,
    pub sell_price: u64,
}

/// A vendor: a price list plus a till (its own [`Wallet`]) in a single
/// currency `C`.
#[derive(Clone, Debug)]
pub struct Shop<K: Ord + Clone, C: Ord + Clone> {
    listings: BTreeMap<K, Listing>,
    till: Wallet<C>,
    currency: C,
}

impl<K: Ord + Clone, C: Ord + Clone> Shop<K, C> {
    /// Create an empty shop trading in `currency`, with an empty till.
    pub fn new(currency: C) -> Self {
        Shop {
            listings: BTreeMap::new(),
            till: Wallet::new(),
            currency,
        }
    }

    /// Add or replace the listing for `item`.
    pub fn list(&mut self, item: K, buy_price: u64, sell_price: u64) {
        self.listings.insert(
            item,
            Listing {
                buy_price,
                sell_price,
            },
        );
    }

    /// Remove `item` from the price list, returning its former [`Listing`] if
    /// it was listed.
    pub fn delist(&mut self, item: K) -> Option<Listing> {
        self.listings.remove(&item)
    }

    /// `true` if `item` currently has a listing.
    pub fn is_listed(&self, item: K) -> bool {
        self.listings.contains_key(&item)
    }

    /// The price a buyer pays for `item`, or `None` if unlisted.
    pub fn buy_price(&self, item: K) -> Option<u64> {
        self.listings.get(&item).map(|l| l.buy_price)
    }

    /// The price the shop pays back for `item`, or `None` if unlisted.
    pub fn sell_price(&self, item: K) -> Option<u64> {
        self.listings.get(&item).map(|l| l.sell_price)
    }

    /// Deposit `amount` of the shop's currency into its till (e.g. initial
    /// capital, or a scripted restock).
    pub fn stock(&mut self, amount: u64) {
        self.till.deposit(self.currency.clone(), amount);
    }

    /// The shop's current till balance.
    pub fn till_balance(&self) -> u64 {
        self.till.balance(self.currency.clone())
    }

    /// Withdraw `amount` from the till (e.g. a scripted heist or tax),
    /// all-or-nothing. Returns `true` on success.
    pub fn drain_till(&mut self, amount: u64) -> bool {
        self.till.withdraw(self.currency.clone(), amount)
    }

    /// `true` if `item` is listed and `buyer` can afford its buy price.
    pub fn can_buy(&self, buyer: &Wallet<C>, item: K) -> bool {
        match self.buy_price(item) {
            Some(price) => buyer.can_afford(self.currency.clone(), price),
            None => false,
        }
    }

    /// Buy `item` from the shop: `buyer` pays `buy_price` into the till,
    /// atomically. Fails (no change to either wallet) if `item` is unlisted or
    /// `buyer` cannot afford it.
    pub fn buy(&mut self, buyer: &mut Wallet<C>, item: K) -> bool {
        let Some(price) = self.buy_price(item) else {
            return false;
        };
        buyer.transfer(&mut self.till, self.currency.clone(), price)
    }

    /// `true` if `item` is listed and the till can afford to buy it back.
    pub fn can_sell(&self, item: K) -> bool {
        match self.sell_price(item) {
            Some(price) => self.till.can_afford(self.currency.clone(), price),
            None => false,
        }
    }

    /// Sell `item` to the shop: the till pays `sell_price` into `seller`,
    /// atomically. Fails (no change to either wallet) if `item` is unlisted or
    /// the till cannot afford the payout.
    pub fn sell(&mut self, seller: &mut Wallet<C>, item: K) -> bool {
        let Some(price) = self.sell_price(item) else {
            return false;
        };
        self.till.transfer(seller, self.currency.clone(), price)
    }

    /// The number of distinct items currently listed.
    pub fn len(&self) -> usize {
        self.listings.len()
    }

    /// `true` if the shop has no listings.
    pub fn is_empty(&self) -> bool {
        self.listings.is_empty()
    }

    /// Iterate over `(item, listing)` pairs in ascending item order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &Listing)> {
        self.listings.iter()
    }
}

impl<K: Ord + Clone + DetHash, C: Ord + Clone + DetHash> DetHash for Shop<K, C> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.listings.len() as u32);
        for (item, listing) in &self.listings {
            item.det_hash(hasher);
            hasher.write_u64(listing.buy_price);
            hasher.write_u64(listing.sell_price);
        }
        self.currency.det_hash(hasher);
        self.till.det_hash(hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let shop: Shop<u32, u32> = Shop::new(1);
        assert!(shop.is_empty());
        assert_eq!(shop.len(), 0);
        assert_eq!(shop.till_balance(), 0);
    }

    #[test]
    fn test_list_and_prices() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        assert!(shop.is_listed(10));
        assert_eq!(shop.buy_price(10), Some(100));
        assert_eq!(shop.sell_price(10), Some(40));
        assert_eq!(shop.buy_price(999), None, "unlisted item has no price");
    }

    #[test]
    fn test_delist_removes_and_returns_listing() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        let removed = shop.delist(10);
        assert_eq!(
            removed,
            Some(Listing {
                buy_price: 100,
                sell_price: 40
            })
        );
        assert!(!shop.is_listed(10));
        assert_eq!(shop.delist(10), None, "second delist returns None");
    }

    #[test]
    fn test_relist_replaces_old_listing() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        shop.list(10, 200, 80);
        assert_eq!(shop.buy_price(10), Some(200));
        assert_eq!(shop.sell_price(10), Some(80));
        assert_eq!(shop.len(), 1, "relisting does not duplicate");
    }

    #[test]
    fn test_stock_and_till_balance() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.stock(500);
        assert_eq!(shop.till_balance(), 500);
        shop.stock(100);
        assert_eq!(shop.till_balance(), 600);
    }

    #[test]
    fn test_drain_till_all_or_nothing() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.stock(100);
        assert!(shop.drain_till(60));
        assert_eq!(shop.till_balance(), 40);
        assert!(!shop.drain_till(1000), "cannot drain more than balance");
        assert_eq!(shop.till_balance(), 40, "failed drain leaves till unchanged");
    }

    #[test]
    fn test_can_buy_reflects_affordability_and_listing() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        let mut buyer: Wallet<u32> = Wallet::new();
        buyer.deposit(1, 50);
        assert!(!shop.can_buy(&buyer, 10), "cannot afford 100 with 50");
        buyer.deposit(1, 60);
        assert!(shop.can_buy(&buyer, 10), "now affords 110 >= 100");
        assert!(!shop.can_buy(&buyer, 999), "unlisted item is never buyable");
    }

    #[test]
    fn test_buy_success_transfers_atomically() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        let mut buyer: Wallet<u32> = Wallet::new();
        buyer.deposit(1, 150);
        assert!(shop.buy(&mut buyer, 10));
        assert_eq!(buyer.balance(1), 50);
        assert_eq!(shop.till_balance(), 100);
    }

    #[test]
    fn test_buy_unlisted_item_fails_no_change() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        let mut buyer: Wallet<u32> = Wallet::new();
        buyer.deposit(1, 150);
        assert!(!shop.buy(&mut buyer, 999));
        assert_eq!(buyer.balance(1), 150, "buyer unchanged on unlisted item");
        assert_eq!(shop.till_balance(), 0, "till unchanged on unlisted item");
    }

    #[test]
    fn test_buy_unaffordable_fails_no_change() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        let mut buyer: Wallet<u32> = Wallet::new();
        buyer.deposit(1, 50);
        assert!(!shop.buy(&mut buyer, 10));
        assert_eq!(buyer.balance(1), 50, "buyer unchanged on failed buy");
        assert_eq!(shop.till_balance(), 0, "till unchanged on failed buy");
    }

    #[test]
    fn test_can_sell_reflects_till_funding() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        assert!(!shop.can_sell(10), "empty till cannot buy back");
        shop.stock(40);
        assert!(shop.can_sell(10));
        assert!(!shop.can_sell(999), "unlisted item is never sellable");
    }

    #[test]
    fn test_sell_success_pays_seller_from_till() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        shop.stock(500);
        let mut seller: Wallet<u32> = Wallet::new();
        assert!(shop.sell(&mut seller, 10));
        assert_eq!(seller.balance(1), 40);
        assert_eq!(shop.till_balance(), 460);
    }

    #[test]
    fn test_sell_underfunded_till_fails_no_change() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        shop.stock(10); // not enough to pay 40
        let mut seller: Wallet<u32> = Wallet::new();
        assert!(!shop.sell(&mut seller, 10));
        assert_eq!(seller.balance(1), 0, "seller unchanged on failed sell");
        assert_eq!(shop.till_balance(), 10, "till unchanged on failed sell");
    }

    #[test]
    fn test_sell_unlisted_item_fails() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.stock(500);
        let mut seller: Wallet<u32> = Wallet::new();
        assert!(!shop.sell(&mut seller, 999));
        assert_eq!(shop.till_balance(), 500);
    }

    #[test]
    fn test_buy_then_sell_round_trip_conserves_total_currency() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 100, 40);
        shop.stock(500);
        let mut player: Wallet<u32> = Wallet::new();
        player.deposit(1, 150);
        let total_before = player.balance(1) + shop.till_balance();

        assert!(shop.buy(&mut player, 10));
        assert_eq!(
            player.balance(1) + shop.till_balance(),
            total_before,
            "buy conserves total currency"
        );

        assert!(shop.sell(&mut player, 10));
        assert_eq!(
            player.balance(1) + shop.till_balance(),
            total_before,
            "sell conserves total currency"
        );
    }

    #[test]
    fn test_zero_price_listing_buy_is_free_no_draw_needed() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(10, 0, 0);
        let mut buyer: Wallet<u32> = Wallet::new();
        assert!(shop.buy(&mut buyer, 10), "0-cost item is always buyable");
        assert_eq!(buyer.balance(1), 0);
    }

    #[test]
    fn test_iter_sorted_by_item() {
        let mut shop: Shop<u32, u32> = Shop::new(1);
        shop.list(30, 1, 1);
        shop.list(10, 2, 2);
        shop.list(20, 3, 3);
        let items: Vec<u32> = shop.iter().map(|(k, _)| *k).collect();
        assert_eq!(items, vec![10, 20, 30], "iteration in ascending item order");
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a: Shop<u32, u32> = Shop::new(1);
        a.list(10, 100, 40);
        a.list(20, 200, 80);
        a.stock(500);

        let mut b: Shop<u32, u32> = Shop::new(1);
        b.list(20, 200, 80);
        b.list(10, 100, 40);
        b.stock(500);

        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "listing insertion order does not affect the hash"
        );

        let mut c: Shop<u32, u32> = Shop::new(1);
        c.list(10, 100, 40);
        c.list(20, 200, 81); // one different sell/buy price
        c.stock(500);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "different price → different hash"
        );
    }

    #[test]
    fn test_det_hash_sensitive_to_till_balance() {
        let mut a: Shop<u32, u32> = Shop::new(1);
        a.list(10, 100, 40);
        a.stock(500);

        let mut b: Shop<u32, u32> = Shop::new(1);
        b.list(10, 100, 40);
        b.stock(600);

        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "different till balance → different hash"
        );
    }
}
