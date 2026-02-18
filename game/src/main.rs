//! # GUR — entry point
//!
//! Bootstraps logging and engine plugins, then hands control to the
//! winit event loop via `GurApp`.

mod platform;
mod scene;

use anyhow::Result;
use winit::event_loop::{ControlFlow, EventLoop};

use gur_core::{
    ecs::GurWorld,
    event::EventBus,
    plugin::PluginRegistry,
    scheduler::Schedule,
    time::{Time, FixedTime},
};

use gur_assets::plugin::AssetsPlugin;
use gur_audio::plugin::AudioPlugin;
use gur_combat::plugin::CombatPlugin;
use gur_input::plugin::InputPlugin;
use gur_narrative::plugin::NarrativePlugin;
use gur_physics::plugin::PhysicsPlugin;
use gur_render::plugin::RenderPlugin;
use gur_save::plugin::SavePlugin;
use gur_scripting::plugin::ScriptingPlugin;
use gur_tools::plugin::ToolsPlugin;
use gur_world::{plugin::WorldPlugin, Zone, ZoneRegistry};
use gur_render::Color;

use platform::{GurApp, PlatformConfig};

fn main() -> Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    ).init();

    log::info!("╔══════════════════════════════════╗");
    log::info!("║  GUR Engine  v{}             ║", env!("CARGO_PKG_VERSION"));
    log::info!("╚══════════════════════════════════╝");

    // ── Engine sub-systems (plugins run their own init, independently of
    //    the game-loop scene which lives inside GurApp). ───────────────────
    let mut world    = GurWorld::new();
    let mut schedule = Schedule::new();
    let mut bus      = EventBus::new();

    world.insert_resource(Time::new());
    world.insert_resource(FixedTime::sixty_hz());

    let mut plugins = PluginRegistry::new();
    plugins
        .add(AssetsPlugin::default())?
        .add(RenderPlugin::default())?
        .add(AudioPlugin::default())?
        .add(InputPlugin::default())?
        .add(PhysicsPlugin::default())?
        .add(CombatPlugin::default())?
        .add(WorldPlugin::default())?
        .add(SavePlugin::default())?
        .add(ScriptingPlugin::default())?
        .add(NarrativePlugin::default())?
        .add(ToolsPlugin::default())?;

    plugins.build_all(&mut world, &mut schedule)?;

    // Register the Ashlands zone
    {
        let registry = world.resource_mut::<ZoneRegistry>();
        registry.register(Zone {
            id:                "ashlands".to_string(),
            display_name:      "The Ashlands".to_string(),
            tilemap_path:      "maps/ashlands.tmx".to_string(),
            ambient_color:     Color::ORANGE,
            ambient_intensity: 0.6,
            music_path:        Some("audio/ashlands.ogg".to_string()),
            enemy_spawns:      vec!["ember_knight".to_string()],
            transitions:       vec![],
        });
        registry.set_active("ashlands");
    }

    // Suppress unused-variable warnings — world/schedule/bus will feed
    // future ECS systems once the game expands.
    let _ = (world, schedule, bus);

    log::info!("Engine ready — opening window...");

    // ── Winit event loop ──────────────────────────────────────────────────
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = GurApp::new(PlatformConfig {
        title:        "GUR — The Ashlands".to_string(),
        window_width:  1280,
        window_height: 720,
    });

    event_loop.run_app(&mut app)?;

    log::info!("GUR shut down cleanly.");
    Ok(())
}
