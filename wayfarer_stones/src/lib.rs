//! Wayfarer Stones in the game: the edges between the game (`mod_api`) and the rules (`rules.rs`).
//!
//! It has two halves, one on each side of a mod message. The server half (`server.rs`) runs where the
//! world is simulated (a dedicated server, a LAN host, single player's server thread): the state of the
//! open world, its files, and travels, which are queued and made at the end of a server tick, as are the
//! blocks and effects that go with them. The client half (`screen.rs`) runs in the window: the screen,
//! which draws and hears input in hooks that may only read, so what it asks for waits in `Ui::outbox` for
//! the client's tick.

mod rules;
mod screen;
mod server;

use mod_api::{
    BlockChangedEvent, BlockPart, BlockState, BlocksEvent, BreakBlockEvent, ClientHost,
    ClientTickPostEvent, Flow, Hooks, HostSide, InteractionResult, ItemPart, ItemsEvent, LevelHost,
    LevelSide, OnBlockPlacedByEvent, RegisterCommandsEvent, Registrar, RegistrationContext,
    RightClickBlockEvent, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ScreenRenderPostEvent,
    ServerHost, ServerStoppedEvent, ServerTickEvent, StateForPlacementEvent, UseItemEvent,
    UseOnEvent,
};

#[derive(Default)]
struct Mod;

#[mod_api::sunrise_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        server::register(r);
        let keys = server::keys();
        r.block_part(keys.block, StoneBlock);
        r.item_part(keys.stone_item, StoneItem);
        r.item_part(keys.scroll, Scroll);
    }

    fn blocks(&self, _: &RegistrationContext, event: &mut BlocksEvent<'_>) {
        server::blocks(event);
    }

    fn items(&self, _: &RegistrationContext, event: &mut ItemsEvent<'_>) {
        server::items(event);
    }

    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        server::commands(event.commands);
    }

    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        if server::right_click(host, event.click.player, event.click.pos) {
            *event.cancellation_result = InteractionResult::Success;
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        if server::destroyed(host, event.pos) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn block_changed(&self, level: &dyn LevelHost, event: &mut BlockChangedEvent) {
        server::block_changed(level, event);
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, mod_api::ServerTickPhase::ServerTickEnd { .. }) {
            server::tick(host);
        }
    }

    fn server_stopped(&self, _: &mut dyn ServerHost, _: &mut ServerStoppedEvent) {
        server::close_world();
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        screen::render(host, event);
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        screen::flow(screen::mouse_pressed(host, event.screen, event.button))
    }

    fn screen_mouse_scrolled_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseScrolledPreEvent<'_>,
    ) -> Flow {
        screen::flow(screen::mouse_scrolled(host, event.screen, event.scroll))
    }

    fn screen_key_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        screen::flow(screen::key_pressed(host, event.screen, event.key))
    }

    fn screen_char_typed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        screen::flow(screen::char_typed(host, event.screen, event.c))
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        screen::tick(host);
    }
}

/// The stone's lower block.
struct StoneBlock;

#[mod_api::sunrise_part]
impl BlockPart for StoneBlock {
    /// A stone needs room for its upper block: none where there is none.
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        if server::room_above(level, event.context.pos) {
            None
        } else {
            Some(None)
        }
    }
}

/// The stone's item: the owner's limit before it is placed, and the registry once it is.
struct StoneItem;

#[mod_api::sunrise_part]
impl ItemPart for StoneItem {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        (!server::may_place(host, event.player)).then_some(InteractionResult::Fail)
    }

    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent) {
        if let Some(player) = event.placer {
            server::placed(host, player, event.pos);
        }
    }
}

/// The wayfarer's scroll: opens the screen from anywhere.
struct Scroll;

#[mod_api::sunrise_part]
impl ItemPart for Scroll {
    fn use_item(&self, host: HostSide<'_>, event: &mut UseItemEvent) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        server::use_scroll(host, event.player).then_some(InteractionResult::Success)
    }
}
