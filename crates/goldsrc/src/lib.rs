//! # GoldSrc Facade Meta-Crate
//!
//! One-stop ergonomic facade for the next-generation GoldSrc.rs plugin development platform.

pub use goldsrc_api as api;
pub use goldsrc_macros as macros;
pub use goldsrc_spi as spi;

pub use goldsrc_api::*;
pub use goldsrc_macros::{
    ConfigModel, bundle, command, command_prefix, event, menu_action, on_frame, on_load, on_unload,
    permission, permissions, plugin, requires, role, system,
};

/// Developer prelude for GoldSrc plugins.
pub mod prelude {
    pub use goldsrc_api::cvar::{ConfigModel, Cvar, CvarFlags};
    pub use goldsrc_api::hud::{HudColor, HudCoord, HudEffect, HudKind, HudMessage, HudMessageBuilder};
    pub use goldsrc_api::menu::{
        ClassicMenuRenderer, DhudMenuRenderer, Menu, MenuActionHandler, MenuActionRegistry,
        MenuBuilder, MenuContext, MenuItem, MenuPageBuilder, MenuRenderer, MenuRendererKind,
        MenuStyle, RenderedMenuPage,
    };
    pub use goldsrc_api::modifiers::{CommutativeModifier, ModifierContribution};
    pub use goldsrc_api::pipeline::{Pipeline, PipelineFlow};
    pub use goldsrc_api::spec::{Spec, SpecError};
    pub use goldsrc_api::{
        Action, AdminCaps, Alive, All, Angles, AntiSpamAction, Any, Armor, AsLangCode, Auth,
        BlackboardValue, Bot, CancellationToken, CapExpr, ChatScope, ChatTarget, CheckCapability,
        Classname, Client, ClientExt, ClientKind, Command, CommandBuilder, CommandContext,
        CommandError, CommandHandler, CommandRegistry, CommandResult, CommandTarget, Condition,
        Connected, ConnectedClient, ConnectionState, Dead, DeadPlayer, DenyAction, DenyPolicy,
        Dormant, Entity, EntityExt, EntityId, Event, EventHandler, EventPhase, EventRegistry,
        EventSubscriberBuilder, EventSubscription, ExitBehavior, Feedback, FromArg, Health, Hltv,
        Human, HumanClient, Interceptor, ItemKind, ItemTitle, LifeState, LivingHuman, LivingPlayer,
        NodeBuilder, NoneOf, Not, OrderNode, Origin, Phase, PhasedDag, Placeholder,
        PlaceholderBuilder, PlaceholderCall, PlaceholderHandler, PlaceholderMetadata,
        PlaceholderRegistry, Player, PlayerAction, PlayerExt, PlayerSlot, PlayerStateFilter,
        PluginTier, PrintTarget, Prop, PropGet, PropSet, RefineExt, Refined, SlotAction, Solid,
        SolidEntity, Spawned, SpawnedEntity, SpectatingPlayer, Spectator, Team, TeamTarget,
        TypedBlackboard, ValidationResult, Vector3, Velocity, VipCaps, VisualDeny,
    };
    pub use goldsrc_macros::{
        ConfigModel as CvarConfigModel, bundle, command, command_prefix, event, menu_action,
        on_frame, on_load, on_unload, permission, permissions, plugin, requires, role, system,
    };
}
