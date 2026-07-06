//! Hello — the shortest possible IZANAGI program.
//!
//! This is the entire user code. No setup. No config. Run it.

use izanagi::Engine;

fn main() {
    Engine::new()
        .run(|e| {
            if e.frame() == 0 {
                println!("Hello from IZANAGI!");
            }
            if e.frame() >= 60 {
                e.quit();
            }
        })
        .unwrap();
    println!("ran 60 frames");
}
