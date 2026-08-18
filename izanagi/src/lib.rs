//! # IZANAGI
//!
//! A Rust game engine that just works.
//!
//! ```no_run
//! use izanagi::Engine;
//!
//! fn main() {
//!     Engine::new().run(|e| {
//!         // your game, every frame
//!         if e.frame() > 60 { e.quit(); }
//!     });
//! }
//! ```
//!
//! ## Philosophy
//!
//! One type: [`Engine`]. Everything else hangs off it.
//!
//! - No builder. No config. No plugins.
//! - Zero external dependencies.
//! - Headless by default; pluggable backends for windows and terminals.
//! - Deterministic when you want it to be.

#![forbid(unsafe_code)]
// Publication-grade doc hygiene (see izanagi_kit for the same policy): every
// public item documented, all intra-doc links resolve, enforced as `deny`.
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
// No panicking paths in shipped code — the same gate the kit carries. The
// engine had exactly two (a downcast that cannot fail and a stack that cannot
// be empty); both are now expressed so the situation is unrepresentable rather
// than asserted. `not(test)` because a panicking assertion in a test *is* the
// test failing.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod assets;
mod audio;
mod ecs;
mod input;
mod render;
mod time;

pub mod backend;
pub mod collide;
pub mod ease;
pub mod error;
pub mod math;
pub mod rng;
pub mod save;
pub mod scene;
pub mod state;
pub mod tween;

pub mod audio_pcm;
pub mod camera;
pub mod debug;
pub mod event;
pub mod gamepad;
pub mod log;
pub mod sprite;
pub mod tilemap;

pub use assets::{Assets, Handle};
pub use audio::{Audio, Voice};
pub use ecs::{Entity, World};
pub use error::{Error, Result};
pub use input::{Input, Key};
pub use math::{Mat3, Rect, Vec2, Vec3};
pub use render::{Color, Draw, Render};
pub use rng::Rng;
pub use scene::{Node, Scene};
pub use time::Time;

use backend::{Backend, NullBackend};

/// The engine. Your entire game runs through this.
pub struct Engine {
    /// Entity-component-system world.
    pub world: World,
    /// Input state (keyboard, mouse).
    pub input: Input,
    /// Frame timing.
    pub time: Time,
    /// Asset loader.
    pub assets: Assets,
    /// Audio mixer.
    pub audio: Audio,
    /// Renderer.
    pub render: Render,
    /// Scene graph (parent-child transforms).
    pub scene: Scene,
    /// Deterministic random number generator.
    pub rng: Rng,
    /// Up to 4 gamepads.
    pub gamepads: gamepad::Gamepads,
    /// Frame-rate and timing metrics.
    pub metrics: debug::Metrics,

    backend: Box<dyn Backend>,
    frame: u64,
    running: bool,
}

impl Engine {
    /// Create a headless engine. Ideal for tests, CI, and servers.
    pub fn new() -> Self {
        Self::with_backend(Box::new(NullBackend::new()))
    }

    /// Create an engine that renders to the terminal via ANSI escape codes.
    pub fn terminal() -> Self {
        Self::with_backend(Box::new(backend::TerminalBackend::new()))
    }

    /// Create an engine with a custom [`Backend`].
    pub fn with_backend(backend: Box<dyn Backend>) -> Self {
        Self {
            world: World::new(),
            input: Input::new(),
            time: Time::new(),
            assets: Assets::new(),
            audio: Audio::new(),
            render: Render::new(),
            scene: Scene::new(),
            rng: Rng::default(),
            gamepads: gamepad::Gamepads::new(),
            metrics: debug::Metrics::default(),
            backend,
            frame: 0,
            running: false,
        }
    }

    /// Seed the RNG for deterministic runs.
    pub fn seed(mut self, seed: u64) -> Self {
        self.rng = Rng::new(seed);
        self
    }

    /// Run your game until the backend requests quit (or [`Engine::quit`]).
    pub fn run<F: FnMut(&mut Engine)>(mut self, mut tick: F) -> Result<()> {
        self.backend.init()?;
        self.running = true;

        while self.running {
            if !self.backend.poll(&mut self.input) {
                self.running = false;
                break;
            }

            let dt = self.backend.dt();
            self.time.advance(dt);
            self.metrics.record(dt);

            tick(&mut self);

            let (clear, draws, texts) = self.render.drain();
            self.backend.present(clear, &draws, &texts);

            self.frame += 1;
            self.input.end_frame();
            self.gamepads.end_frame();
        }

        self.backend.shutdown();
        Ok(())
    }

    /// Stop the engine at the end of this frame.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Current frame number, starting at 0.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Seconds elapsed since `run` started.
    pub fn elapsed(&self) -> f32 {
        self.time.elapsed()
    }

    /// Delta time of the last frame in seconds.
    pub fn dt(&self) -> f32 {
        self.time.dt()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Install a process-wide panic hook that logs the panic to the engine's
/// log channel and a backtrace location.
///
/// Call once early in `main()` for production builds — by default Rust's
/// panic hook just prints to stderr.
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            String::from("(non-string panic payload)")
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown>".to_string());
        crate::log::_emit(crate::log::Level::Error, format_args!("PANIC at {}: {}", location, msg));
        prev(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_constructs() {
        let e = Engine::new();
        assert_eq!(e.frame(), 0);
        assert_eq!(e.elapsed(), 0.0);
    }

    #[test]
    fn run_quits_immediately() {
        Engine::new().run(|e| e.quit()).unwrap();
    }

    #[test]
    fn run_respects_backend_frame_budget() {
        let backend = Box::new(NullBackend::new().with_frames(10));
        let mut observed = 0u64;
        Engine::with_backend(backend)
            .run(|e| {
                observed = e.frame();
            })
            .unwrap();
        assert_eq!(observed, 9);
    }

    #[test]
    fn default_matches_new() {
        let a = Engine::default();
        let b = Engine::new();
        assert_eq!(a.frame(), b.frame());
    }

    #[test]
    fn seed_is_deterministic() {
        let a = Engine::new().seed(12345).rng.clone();
        let b = Engine::new().seed(12345).rng.clone();
        // Clone twice, draw same sequence.
        let mut ac = a;
        let mut bc = b;
        for _ in 0..10 {
            assert_eq!(ac.u64(), bc.u64());
        }
    }
}
