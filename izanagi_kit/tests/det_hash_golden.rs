//! Golden-value regression guard for the `DetHash` **wire format**.
//!
//! ## Why this exists
//!
//! `izanagi_kit`'s headline guarantee is *bit-identical* state hashing across
//! platforms and builds — the foundation of replay, lockstep netcode, and
//! save-file checksums. `tests/determinism.rs` pins one end-to-end simulation,
//! but that sim only exercises a handful of types (`EntityAllocator`,
//! `SparseSet`, `Fixed`, `SplitMix64`). Most other `DetHash`
//! implementations are pinned nowhere — the exact set is enumerated and kept
//! current by `UNPINNED_DET_HASH` in `tests/global_invariants_hold.rs`, which
//! fails if an impl is neither pinned here nor declared there.
//!
//! The per-module unit tests only assert `hash(x) == hash(x)` (self-consistency)
//! and `hash(x) != hash(y)` (discrimination). **Both survive a wire-format
//! change** — if a refactor reorders fields in a `det_hash` impl, or swaps a
//! `write_u32` for a `write_u64`, both sides of those assertions move together
//! and the tests still pass. The breakage is invisible until an *old* replay or
//! save (hashed by a previous build) is loaded against the new format.
//!
//! This file converts that "stability by discipline" into a mechanical
//! tripwire: it constructs a fixed, representative instance of each public
//! `DetHash` type and pins its exact `hash_state` value. Any change to a
//! wire format flips exactly one line here, forcing a deliberate decision
//! ("did we mean to break replay compatibility?") instead of a silent
//! regression.
//!
//! ## Regenerating after an intentional format change
//!
//! Run `cargo test --test det_hash_golden print_golden -- --ignored --nocapture`,
//! copy the printed table into `EXPECTED`, and document the break in
//! `CHANGELOG.md`. Treat any *unexpected* diff as a determinism regression.

use izanagi_kit::{
    ability::{Ability, AbilitySet},
    behavior::{BehaviorNode, BehaviorStatus, BehaviorTree},
    content::Color,
    generate_dungeon,
    geometry::Distance,
    hash_state,
    hexgrid::Hex,
    voronoi::{voronoi_partition, VoronoiGrid},
    wfc_solve, Aabb, Affix, AffixedItem, ArchTable, AssetStore, BarWidget, Calendar, Camera, Cell,
    ChangeTracker, Changed, CmdQueue, ComponentEvent, Connector, Cooldown, DamageType, Dialogue,
    DialogueNode, Dice, Dungeon, EncounterPack, EntityAllocator, EquipSlot, Equipment, EventLog,
    EventQueue, FactionMap, Fixed, Fsm, GenParams, HFsm, HudPanel, Identification, InfluenceMap,
    Ingredient, InputBuffer, Inventory, LayeredMap, LevelCurve, LightMap, Menu, MetaProgress,
    MsgLog, MultiMap, NetInputBuffer, Objective, Observed, PassabilityGrid, Pool, Position,
    Profiler, Progression, Quest, RandomTable, Recipe, Rect, Relations, Render, ResistanceProfile,
    Screen, Shop, ShuffleBag, SimpleTileTable, SpatialHash, SplitMix64, StatLine, Stats,
    StatsModifier, StatusSet, ThreatTable, TileMap, TimerQueue, Trigger, Tween, TweenSequence,
    Vec2, Vec3, Visibility, VisibilityMap, Wallet, WfcGrid, WfcResult, WfcRules, Xoshiro256pp,
};
use std::collections::{BTreeMap, BTreeSet, LinkedList, VecDeque};

/// Build the canonical (name, golden-hash) table. Each entry constructs a
/// fixed instance whose `hash_state` must never change without a deliberate,
/// documented format break. Keep names stable and unique — they are the diff
/// anchors.
fn cases() -> Vec<(&'static str, u64)> {
    // A deterministic entity (index 0, generation 0).
    let mut alloc = EntityAllocator::new();
    let e0 = alloc.allocate();
    let e1 = alloc.allocate();

    // Relations: e1 parented to e0.
    let mut rel = Relations::new();
    rel.attach(e1, e0);

    // Message log with two entries.
    let mut log = MsgLog::new(8);
    log.push("hello");
    log.push("world");

    // Hierarchical FSM over plain integer states/events.
    let hfsm: HFsm<u32, u8> = HFsm::new(1)
        .with_parent(2, 1)
        .on(1, 10u8, 2)
        .on_any(20u8, 3);

    // Ability set: one keyed ability with an integer effect payload.
    let abilities: AbilitySet<u32, u32> =
        AbilitySet::new().with(1, Ability::new("Firebolt", 5, 3, 6, 42u32));

    // Behavior tree: a sequence of two action leaves.
    let bt: BehaviorTree<u32> = BehaviorTree::new(BehaviorNode::sequence(vec![
        BehaviorNode::action(1),
        BehaviorNode::action(2),
    ]));

    // Timer queue: one one-shot and one recurring entry. Pins the `period`
    // field whose omission was the Round 10 bug — a re-omission flips this.
    let mut timers: TimerQueue<u32> = TimerQueue::new();
    timers.schedule(5, 99);
    timers.schedule_repeat(5, 3, 99);

    // A small fixed-seed dungeon. Pins the `rooms` field (Round 11) plus the
    // tile bitmap; a re-omission of rooms — or a generation/wire change — flips it.
    let dungeon: Dungeon = {
        let mut rng = SplitMix64::new(0xD17A9E);
        generate_dungeon(24, 16, &mut rng, GenParams::default())
    };

    // Voronoi partition of a fixed 8x8 with two seeds. Pins the per-cell
    // owner/distance vectors' hash order — a change to `cells`/`dist`
    // write order flips this.
    let voronoi: VoronoiGrid = voronoi_partition(&[(1, 1), (6, 3)], 8, 8, Distance::Manhattan);

    // A two-floor multi-map with one connector. Pins the `floors` field
    // (Round 12) alongside current_floor and connectors.
    let multimap: MultiMap = {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        let f0 = generate_dungeon(20, 12, &mut a, GenParams::default());
        let f1 = generate_dungeon(20, 12, &mut b, GenParams::default());
        let mut m = MultiMap::new(vec![f0, f1], 0);
        m.add_connector(Connector {
            from_floor: 0,
            from_x: 3,
            from_y: 3,
            to_floor: 1,
            to_x: 5,
            to_y: 5,
        });
        m
    };

    // ── std-type wire formats ────────────────────────────────────────────────
    // Every `DetHash` impl added in Round 66 gets a pinned byte pattern too:
    // widening `u8`→`u32` writes or re-keying a map silently shifts every
    // downstream digest.
    let mut vd: VecDeque<u32> = VecDeque::new();
    vd.push_back(7);
    vd.push_back(9);
    let mut ll: LinkedList<u16> = LinkedList::new();
    ll.push_back(3);
    ll.push_back(5);
    let mut btree_map: BTreeMap<u16, u32> = BTreeMap::new();
    btree_map.insert(2, 20);
    btree_map.insert(1, 10); // sorted iteration is the canonical order
    let mut btree_set: BTreeSet<u16> = BTreeSet::new();
    btree_set.insert(9);
    btree_set.insert(4);
    let referent = 41u32;
    let borrow: &u32 = &referent;
    let boxed: Box<u32> = Box::new(17);

    // ── kit sim-state types ──────────────────────────────────────────────────
    // Deterministic seeded instances. Each fixture carries at least one
    // non-default field so the pin discriminates the wire layout, not just
    // an empty structure.
    let mut xoshiro = Xoshiro256pp::new(0xC0FFEE);
    xoshiro.next_u64();
    xoshiro.next_u64();

    let mut tilemap: TileMap<u8> = TileMap::new(8, 6, 0);
    tilemap.set(1, 1, 9);
    tilemap.set(7, 5, 3);
    let mut layered: LayeredMap<u8> = LayeredMap::new(4, 4, 2, 0);
    layered.set(1, 2, 3, 7);
    let mut pass: PassabilityGrid = PassabilityGrid::new(6, 5);
    pass.set_blocked(2, 3, true);
    let mut vis_map: VisibilityMap = VisibilityMap::new(5, 5);
    vis_map.set(1, 2, Visibility::Visible);
    vis_map.set(3, 3, Visibility::Remembered);
    let mut spatial: SpatialHash<u32> = SpatialHash::new(4);
    spatial.insert(7, 10, 20);
    spatial.insert(8, 11, 21);
    let mut influence = InfluenceMap::new(6, 6);
    influence.add_source(2, 2, 10, 3);
    influence.set(0, 0, -4);
    let mut lightmap = LightMap::new(8, 8);
    lightmap.add_light(4, 4, 3, 12);
    let mut factions: FactionMap<u8> = FactionMap::new();
    factions.set(1, 2, -50);
    factions.set_symmetric(0, 3, 20);
    let mut cmds: CmdQueue<u8> = CmdQueue::new();
    cmds.push(9);
    cmds.push(10);
    let mut event_queue: EventQueue<u16> = EventQueue::new();
    event_queue.push(100);
    event_queue.push(101);
    let mut net: NetInputBuffer<u8, u32> = NetInputBuffer::new();
    net.seed(1, 55);
    net.confirm(3, 1, 55);
    let mut inputbuf: InputBuffer<u8> = InputBuffer::new(2, 1);
    inputbuf.press(7);
    inputbuf.tick(2);
    let mut observed: Observed<u32> = Observed::new();
    observed.insert(e0, 77);
    let changed: Changed<u32> = Changed::at(42, 9);
    let mut tracker = ChangeTracker::new();
    tracker.advance();
    tracker.advance();
    tracker.advance();
    let mut statuses: StatusSet<u8> = StatusSet::new();
    statuses.apply(3, 10, 5);
    statuses.apply(5, 2, -1);
    let mut threat: ThreatTable<u8> = ThreatTable::new();
    threat.add(1, 50);
    threat.add(2, 30);
    let pool = Pool::with_current(100, 65, 2);
    let curve = LevelCurve::new(100, 50, 10);
    let progression = Progression::with_xp(curve, 350);
    let mut calendar = Calendar::new(24);
    calendar.advance(50);
    let mut bag_rng = SplitMix64::new(0x5EED);
    let mut bag: ShuffleBag<u8> = ShuffleBag::new(vec![1, 2, 3, 4]);
    bag.draw(&mut bag_rng); // drawing mutates remaining-order state
    let mut wallet: Wallet<u8> = Wallet::new();
    wallet.deposit(0, 500);
    wallet.deposit(1, 1200);
    let mut shop: Shop<u8, u8> = Shop::new(0);
    shop.list(4, 100, 60);
    shop.stock(7);
    let mut equipment: Equipment<u32> = Equipment::new();
    equipment.equip(EquipSlot::MainHand, 41);
    equipment.curse(EquipSlot::OffHand);
    let mut fsm: Fsm<u8, u8> = Fsm::new(0);
    fsm.add_transition(0, 1, 2);
    fsm.add_transition(2, 1, 0);
    fsm.fire(&1);
    let trigger: Trigger<u8, u16> = Trigger::new(3, vec![9, 10]);
    let tween = Tween::with_elapsed(Fixed::from_int(1), Fixed::from_int(9), 10, 4);
    let tween_seq = TweenSequence::new(vec![
        Tween::new(Fixed::ZERO, Fixed::ONE, 5),
        Tween::new(Fixed::ONE, Fixed::from_int(4), 5),
    ]);
    let mut meta: MetaProgress<u8, u8> = MetaProgress::new();
    meta.unlock(2);
    meta.record_best(1, 9000);
    let table: RandomTable<u8> = RandomTable::new().with(3, 10).with(1, 20);
    let pack: EncounterPack<u8> = EncounterPack::new()
        .with_slot(5, 1, 3)
        .with_optional_slot(6, 0, 2, 50);
    let mut quest = Quest::new("Slay rats").with_objective(Objective::new("rats", 5));
    quest.progress(0, 2);
    let recipe: Recipe<u8, u8> = Recipe::new(
        "torch",
        vec![
            Ingredient { key: 2, count: 1 },
            Ingredient { key: 3, count: 2 },
        ],
        9,
    );
    let modifier = StatsModifier {
        attack: 3,
        defense: -1,
        max_hp: 10,
    };
    let mut id_rng = SplitMix64::new(0x1D);
    let mut identification: Identification<u8, u8> =
        Identification::new(&[1, 2, 3], &[10, 20, 30], &mut id_rng);
    identification.identify(2);
    let mut inventory: Inventory<u32> = Inventory::new(4);
    inventory.add(11);
    inventory.add(12);
    let mut menu: Menu<u8> = Menu::new();
    menu.add_item("go", 1);
    menu.add_disabled("no", 2);
    menu.add_item("quit", 3);
    menu.move_down();
    let mut profiler = Profiler::new(4);
    profiler.begin_tick();
    profiler.record("update", 300);
    profiler.record("render", 500);
    let mut event_log: EventLog<u8> = EventLog::new(4);
    event_log.push(3, 9);
    event_log.push(4, 10);
    let mut arch: ArchTable<u8> = ArchTable::new();
    arch.insert(e0, 5);
    arch.insert(e1, 6);
    let mut assets: AssetStore<u32> = AssetStore::new();
    assets.insert(55);
    assets.insert(66);
    let mut autotile: SimpleTileTable = SimpleTileTable::new();
    autotile.set(0b1010, 7);
    autotile.set(0b0101, 8);
    let dialogue = Dialogue::new(
        vec![
            DialogueNode::new("hi").with_choice("bye?", 1),
            DialogueNode::new("bye"),
        ],
        0,
    );
    // Fully-permissive rules cannot contradict, so the grid is deterministic.
    let mut wfc_rules = WfcRules::new(2);
    for a in 0..2u8 {
        for d in 0..4usize {
            for b in 0..2u8 {
                wfc_rules.allow(a, d, b);
            }
        }
    }
    let wfc_grid: WfcGrid = match wfc_solve(4, 4, &wfc_rules, &mut SplitMix64::new(0xFACE)) {
        WfcResult::Ok(g) => g,
        WfcResult::Contradiction => panic!("fully-allowed rules cannot contradict"),
    };

    vec![
        ("Fixed::from_int(3)", hash_state(&Fixed::from_int(3))),
        ("i8(-3)", hash_state(&-3i8)),
        ("i16(-300)", hash_state(&-300i16)),
        ("u128(max/3)", hash_state(&(u128::MAX / 3))),
        ("i128(min/7)", hash_state(&(i128::MIN / 7))),
        ("unit()", hash_state(&())),
        (
            "tuple4(u8,u16,u32,u64)",
            hash_state(&(1u8, 2u16, 3u32, 4u64)),
        ),
        ("array[u16;3]", hash_state(&[5u16, 6, 7])),
        ("Box<u32>(17)", hash_state(&boxed)),
        ("&u32(41)", hash_state(&borrow)),
        ("VecDeque<u32>[7,9]", hash_state(&vd)),
        ("LinkedList<u16>[3,5]", hash_state(&ll)),
        ("BTreeMap<u16,u32>", hash_state(&btree_map)),
        ("BTreeSet<u16>", hash_state(&btree_set)),
        ("Entity(0,0)", hash_state(&e0)),
        (
            "Vec2(2,-5)",
            hash_state(&Vec2::new(Fixed::from_int(2), Fixed::from_int(-5))),
        ),
        (
            "Vec3(1,2,3)",
            hash_state(&Vec3::new(
                Fixed::from_int(1),
                Fixed::from_int(2),
                Fixed::from_int(3),
            )),
        ),
        ("Aabb(1,2,3,4)", hash_state(&Aabb::new(1, 2, 3, 4))),
        (
            "Camera(5,5,16,16,64,64)",
            hash_state(&Camera::new(5, 5, 16, 16, 64, 64)),
        ),
        ("Cooldown(5)", hash_state(&Cooldown::new(5))),
        ("Dice(3d6+1)", hash_state(&Dice::new(3, 6, 1))),
        ("Stats(20,5,2)", hash_state(&Stats::new(20, 5, 2))),
        ("DamageType::Fire", hash_state(&DamageType::Fire)),
        (
            "ResistanceProfile{Fire:50}",
            hash_state(&ResistanceProfile::new().with(DamageType::Fire, 50)),
        ),
        ("Screen(2x2)", hash_state(&Screen::new(2, 2))),
        ("BarWidget(7,10,20)", hash_state(&BarWidget::new(7, 10, 20))),
        ("StatLine(HP,42)", hash_state(&StatLine::new("HP", 42))),
        (
            "HudPanel(0,0,10,5)",
            hash_state(&HudPanel::new(0, 0, 10, 5)),
        ),
        ("Relations{e1->e0}", hash_state(&rel)),
        ("MsgLog[hello,world]", hash_state(&log)),
        ("HFsm<u32,u8>", hash_state(&hfsm)),
        ("AbilitySet<u32,u32>", hash_state(&abilities)),
        ("BehaviorTree<u32>", hash_state(&bt)),
        ("TimerQueue<u32>[5;5r3]", hash_state(&timers)),
        ("Dungeon(24x16,seed)", hash_state(&dungeon)),
        ("MultiMap(2floors,1conn)", hash_state(&multimap)),
        ("VoronoiGrid(8x8,2seeds)", hash_state(&voronoi)),
        ("Hex(2,-3)", hash_state(&Hex::new(2, -3))),
        ("Xoshiro256pp(2draws)", hash_state(&xoshiro)),
        ("TileMap(8x6)", hash_state(&tilemap)),
        ("LayeredMap(4x4x2)", hash_state(&layered)),
        ("PassabilityGrid(6x5)", hash_state(&pass)),
        ("VisibilityMap(5x5)", hash_state(&vis_map)),
        ("SpatialHash(cell4)", hash_state(&spatial)),
        ("InfluenceMap(6x6)", hash_state(&influence)),
        ("LightMap(8x8)", hash_state(&lightmap)),
        ("FactionMap(2rel)", hash_state(&factions)),
        ("CmdQueue[9,10]", hash_state(&cmds)),
        ("EventQueue[100,101]", hash_state(&event_queue)),
        ("NetInputBuffer(seed)", hash_state(&net)),
        ("InputBuffer(press)", hash_state(&inputbuf)),
        ("Observed{e0:77}", hash_state(&observed)),
        ("Changed(42@9)", hash_state(&changed)),
        ("ChangeTracker(t2)", hash_state(&tracker)),
        ("StatusSet(2eff)", hash_state(&statuses)),
        ("ThreatTable(2src)", hash_state(&threat)),
        ("Pool(65/100,r2)", hash_state(&pool)),
        ("LevelCurve(100,50,10)", hash_state(&curve)),
        ("Progression(350xp)", hash_state(&progression)),
        ("Calendar(tpd24)", hash_state(&calendar)),
        ("ShuffleBag(4,1drawn)", hash_state(&bag)),
        ("Wallet(2curr)", hash_state(&wallet)),
        ("Shop(1listing)", hash_state(&shop)),
        ("Equipment(1slot)", hash_state(&equipment)),
        ("Fsm(3st,fired)", hash_state(&fsm)),
        ("Trigger<u8,u16>", hash_state(&trigger)),
        ("Tween(elapsed4)", hash_state(&tween)),
        ("TweenSequence(2)", hash_state(&tween_seq)),
        ("MetaProgress(u1,r1)", hash_state(&meta)),
        ("RandomTable(2w)", hash_state(&table)),
        ("EncounterPack(2slot)", hash_state(&pack)),
        ("Quest(1obj)", hash_state(&quest)),
        ("Objective(rats:5)", hash_state(&Objective::new("o2", 7))),
        ("Recipe(torch)", hash_state(&recipe)),
        ("StatsModifier(+3,-1,10)", hash_state(&modifier)),
        ("Identification(3k)", hash_state(&identification)),
        ("Inventory(2items)", hash_state(&inventory)),
        ("Menu(3items)", hash_state(&menu)),
        ("Profiler(1tick)", hash_state(&profiler)),
        ("EventLog(2ev)", hash_state(&event_log)),
        ("ArchTable(2rows)", hash_state(&arch)),
        ("AssetStore(2)", hash_state(&assets)),
        ("SimpleTileTable(2)", hash_state(&autotile)),
        ("Dialogue(2nodes)", hash_state(&dialogue)),
        ("WfcGrid(4x4)", hash_state(&wfc_grid)),
        ("Position(3,4)", hash_state(&Position { x: 3, y: 4 })),
        (
            "Render(@,gold)",
            hash_state(&Render {
                glyph: '@',
                color: Color::rgb(255, 200, 0),
            }),
        ),
        (
            "Cell(a,fg,bg)",
            hash_state(&Cell {
                glyph: 'a',
                fg: Color::rgb(1, 2, 3),
                bg: Color::rgb(4, 5, 6),
            }),
        ),
        (
            "Rect(5,6,7,8)",
            hash_state(&Rect {
                x: 5,
                y: 6,
                w: 7,
                h: 8,
            }),
        ),
        ("Color::rgb(9,8,7)", hash_state(&Color::rgb(9, 8, 7))),
        // Single-byte discriminants hash identically across enums —
        // Visibility and QuestState are pinned transitively (VisibilityMap
        // cells, Objective.state); keep the one otherwise-uncovered enum.
        (
            "BehaviorStatus::Running",
            hash_state(&BehaviorStatus::Running),
        ),
        (
            "ComponentEvent::Removed",
            hash_state(&ComponentEvent::Removed(e1)),
        ),
        (
            "AffixSlot::Prefix",
            hash_state(&izanagi_kit::AffixSlot::Prefix),
        ),
        (
            "Affix<u8>(prefix)",
            hash_state(&Affix::prefix("Rusty", 3u8)),
        ),
        (
            "AffixedItem<u32,u8>",
            hash_state(&AffixedItem::<u32, u8>::plain(100u32)),
        ),
        (
            "Choice(lbl->1)",
            hash_state(&izanagi_kit::dialogue::Choice::new("go", 1)),
        ),
        ("DialogueNode(1choice)", hash_state(&DialogueNode::new("n"))),
        (
            "Quat(z,90°)",
            hash_state(
                &izanagi_kit::quat::Quat::from_axis_angle(
                    Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE),
                    Fixed::HALF_PI,
                )
                .unwrap(),
            ),
        ),
        ("GCounter(2r)[3,1]", {
            let mut g = izanagi_kit::crdt::GCounter::new(2);
            g.add(0, 3);
            g.add(1, 1);
            hash_state(&g)
        }),
        ("PNCounter(2r)+2-1", {
            let mut p = izanagi_kit::crdt::PNCounter::new(2);
            p.increment(0);
            p.increment(0);
            p.decrement(1);
            hash_state(&p)
        }),
        ("GSet<u32>{1,4}", {
            let mut s = izanagi_kit::crdt::GSet::<u32>::new();
            s.insert(1);
            s.insert(4);
            hash_state(&s)
        }),
        ("TwoPhaseSet<u32>{1,4}-{4}", {
            let mut s = izanagi_kit::crdt::TwoPhaseSet::<u32>::new();
            s.insert(1);
            s.insert(4);
            s.remove(4);
            hash_state(&s)
        }),
    ]
}

/// Pinned golden hashes. A diff in any value is a `DetHash` wire-format change.
/// Regenerate via the `print_golden` helper (see module docs) only for a
/// *deliberate*, changelog-documented break.
const EXPECTED: &[(&str, u64)] = &[
    ("Fixed::from_int(3)", 0x4d28dc7f9dd0f71e),
    ("i8(-3)", 0xaf64704c8602e808),
    ("i16(-300)", 0x0ae7ca07b7386a67),
    ("u128(max/3)", 0xd7e54fa625bb2055),
    ("i128(min/7)", 0xed4b66d5ffab437f),
    ("unit()", 0xcbf29ce484222325),
    ("tuple4(u8,u16,u32,u64)", 0x399de56bff22cb41),
    ("array[u16;3]", 0x2ca0224786d48ed2),
    ("Box<u32>(17)", 0xacd58afcaaf42074),
    ("&u32(41)", 0xadaaa9af328ea9cc),
    ("VecDeque<u32>[7,9]", 0x05e353268e057029),
    ("LinkedList<u16>[3,5]", 0x06c3502b3b4eeed9),
    ("BTreeMap<u16,u32>", 0xfe4ee57d5fecfb0a),
    ("BTreeSet<u16>", 0x66b44022e507df52),
    ("Entity(0,0)", 0xa8c7f832281a39c5),
    ("Vec2(2,-5)", 0x62dee59804c46909),
    ("Vec3(1,2,3)", 0xf4ebf23c75a0eb95),
    ("Aabb(1,2,3,4)", 0x84c39a079fc08121),
    ("Camera(5,5,16,16,64,64)", 0x777750cf734fcf85),
    ("Cooldown(5)", 0x2d401a55eec16520),
    ("Dice(3d6+1)", 0x2fc5364c8c789d11),
    ("Stats(20,5,2)", 0x0a5ee284d3d7fbc2),
    ("DamageType::Fire", 0xaf63bc4c8601b62c),
    ("ResistanceProfile{Fire:50}", 0x25b9589630c86467),
    ("Screen(2x2)", 0x92c965a7dd27d405),
    ("BarWidget(7,10,20)", 0xae1340d60a74329c),
    ("StatLine(HP,42)", 0xb5df64faa084432d),
    ("HudPanel(0,0,10,5)", 0x9e0689ea1f4b9ada),
    ("Relations{e1->e0}", 0x5b5754e32028a8a5),
    ("MsgLog[hello,world]", 0x612662cfb2655e8d),
    ("HFsm<u32,u8>", 0x870c3270a024ab05),
    ("AbilitySet<u32,u32>", 0x6be775165615ef30),
    ("BehaviorTree<u32>", 0x733cfecebb0bc160),
    ("TimerQueue<u32>[5;5r3]", 0x9e3d87f791d59425),
    ("Dungeon(24x16,seed)", 0xe31ab41e7035e685),
    ("MultiMap(2floors,1conn)", 0xa84ad2b8abb52eb8),
    ("VoronoiGrid(8x8,2seeds)", 0xd6994b2321550617),
    ("Hex(2,-3)", 0xafbeef221f187241),
    ("Xoshiro256pp(2draws)", 0xfe9b716c7ebe85fa),
    ("TileMap(8x6)", 0x57102cbb3cc4bfa1),
    ("LayeredMap(4x4x2)", 0xe15a1dfad2d39bd0),
    ("PassabilityGrid(6x5)", 0xeaf3609eb3453717),
    ("VisibilityMap(5x5)", 0xd3bebc9d31758162),
    ("SpatialHash(cell4)", 0x9ddbee2152baa35a),
    ("InfluenceMap(6x6)", 0x804a0a49a978600b),
    ("LightMap(8x8)", 0x94624311b96efba9),
    ("FactionMap(2rel)", 0x58bbd3ffb4500678),
    ("CmdQueue[9,10]", 0xd7ff7106c11e7f34),
    ("EventQueue[100,101]", 0x63974b43383c0c3e),
    ("NetInputBuffer(seed)", 0x423a59222518bcf6),
    ("InputBuffer(press)", 0x9799c4df96c70683),
    ("Observed{e0:77}", 0xffff7e6a0f1cee28),
    ("Changed(42@9)", 0x5f6ad1202fa639d6),
    ("ChangeTracker(t2)", 0xed202287f403d086),
    ("StatusSet(2eff)", 0x1447aeb16f5de928),
    ("ThreatTable(2src)", 0xfa474e4fdc03a678),
    ("Pool(65/100,r2)", 0x5368325e5467ece2),
    ("LevelCurve(100,50,10)", 0x01995d26a95791c9),
    ("Progression(350xp)", 0x2705cfed5c5da50f),
    ("Calendar(tpd24)", 0xeae11dd2741b665f),
    ("ShuffleBag(4,1drawn)", 0x5ddca0891a8ac363),
    ("Wallet(2curr)", 0xd00d670c4adf84c5),
    ("Shop(1listing)", 0xd285d063d0e6cd1e),
    ("Equipment(1slot)", 0x9ebd09637d1949cf),
    ("Fsm(3st,fired)", 0x8d1ace904a398d17),
    ("Trigger<u8,u16>", 0xae7791dd1eea01f5),
    ("Tween(elapsed4)", 0xa05f5d7956adbe03),
    ("TweenSequence(2)", 0xd8e33b8823c15303),
    ("MetaProgress(u1,r1)", 0x356aafb0e9922783),
    ("RandomTable(2w)", 0xf9432d19edfd09eb),
    ("EncounterPack(2slot)", 0x257b8da9d90b7852),
    ("Quest(1obj)", 0xc9ab5b939be2be5e),
    ("Objective(rats:5)", 0x6fa62dab9472efd9),
    ("Recipe(torch)", 0xaa5752711e878668),
    ("StatsModifier(+3,-1,10)", 0x7d12606b03573dc8),
    ("Identification(3k)", 0x4e582d5fe236b1e5),
    ("Inventory(2items)", 0xa442c2ff77c57c07),
    ("Menu(3items)", 0xadca3845a7f1b5f5),
    ("Profiler(1tick)", 0x1ed3ae7f8f8fe2cf),
    ("EventLog(2ev)", 0x2979adb64cbb1c63),
    ("ArchTable(2rows)", 0x6f9e395f3c04b445),
    ("AssetStore(2)", 0x0352f635f476d353),
    ("SimpleTileTable(2)", 0x8c2d3232136e688a),
    ("Dialogue(2nodes)", 0x7361ad892b64cafc),
    ("WfcGrid(4x4)", 0x03d5ba7b578af8a6),
    ("Position(3,4)", 0x47d80f19da3291a2),
    ("Render(@,gold)", 0xd93fe3ac0896a066),
    ("Cell(a,fg,bg)", 0x2685acfde4cb3d13),
    ("Rect(5,6,7,8)", 0x21ae45a89d6cb129),
    ("Color::rgb(9,8,7)", 0x160a9e188e7e3df9),
    ("BehaviorStatus::Running", 0xaf63bf4c8601bb45),
    ("ComponentEvent::Removed", 0xedde65ec42d6cbc4),
    ("AffixSlot::Prefix", 0xaf63bd4c8601b7df),
    ("Affix<u8>(prefix)", 0xdefb89da979553fc),
    ("AffixedItem<u32,u8>", 0x27ca16640b9e97f1),
    ("Choice(lbl->1)", 0xf0dcb985e9a6b4aa),
    ("DialogueNode(1choice)", 0xc6b20827bef5b401),
    ("Quat(z,90°)", 0x086e1ef92e5a3f62),
    ("GCounter(2r)[3,1]", 0xe4dff8725769fc35),
    ("PNCounter(2r)+2-1", 0x752daa6170feaee6),
    ("GSet<u32>{1,4}", 0xa00940f06ee69992),
    ("TwoPhaseSet<u32>{1,4}-{4}", 0x2db8c5a26b323c37),
];

#[test]
fn test_det_hash_wire_format_pinned() {
    let actual = cases();
    assert_eq!(
        actual.len(),
        EXPECTED.len(),
        "case count changed — update EXPECTED (regenerate with the print_golden helper)"
    );
    for ((name_a, hash_a), (name_e, hash_e)) in actual.iter().zip(EXPECTED.iter()) {
        assert_eq!(name_a, name_e, "case ordering/name drifted");
        assert_eq!(
            hash_a, hash_e,
            "DetHash wire format changed for `{name_a}`: \
             0x{hash_e:016x} (pinned) != 0x{hash_a:016x} (now). \
             If intentional, regenerate EXPECTED and document the replay/save break."
        );
    }
}

#[test]
fn test_golden_names_unique() {
    // Names are the diff anchors; duplicates would mask a regression.
    let mut names: Vec<&str> = cases().iter().map(|(n, _)| *n).collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total, "duplicate golden case name");
}

#[test]
fn test_golden_hashes_discriminate() {
    // Distinct fixtures should (overwhelmingly) hash distinctly; a collision
    // here would weaken the tripwire's resolution.
    let hashes: Vec<u64> = cases().iter().map(|(_, h)| *h).collect();
    let total = hashes.len();
    let mut uniq = hashes.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), total, "two golden fixtures collided");
}

/// Prints the current (name, hash) table for pasting into `EXPECTED`.
/// Ignored by default — run explicitly only when regenerating after a
/// deliberate format change.
#[test]
#[ignore]
fn print_golden() {
    println!("const EXPECTED: &[(&str, u64)] = &[");
    for (name, hash) in cases() {
        println!("    ({name:?}, 0x{hash:016x}),");
    }
    println!("];");
}
