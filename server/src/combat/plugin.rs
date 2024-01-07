use bevy::prelude::*;

use super::{
    bundles::*,
    commands::{advance::*, attack::*, block::*, dodge::*, retreat::*, use_skill::*},
    components::*,
    events::*,
    systems::*,
};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Stats>();
        app.register_type::<Attributes>();
        app.register_type::<Status>();
        app.register_type::<Defense>();
        app.register_type::<Offense>();
        app.register_type::<Cooldowns>();
        app.register_type::<HealthRegenTimer>();
        app.register_type::<VigorRegenTimer>();
        app.register_type::<CombatBundle>();

        app.add_event::<CombatEvent>();

        app.add_systems(
            Update,
            (
                (
                    attack,
                    use_skill,
                    start_auto_attacks,
                    handle_auto_attack,
                    stop_auto_attacks,
                    dodge,
                    block,
                    advance,
                    retreat,
                ),
                (
                    on_combat_event_attack,
                    on_combat_event_attempt_hit,
                    on_combat_event_attempt_dodge,
                    on_combat_event_attempt_block,
                    on_combat_event_execute_scripts,
                    on_combat_event_attempt_flee,
                    on_combat_event_apply_damage,
                    on_combat_event_apply_condition,
                    on_combat_event_set_distance,
                    on_combat_event_set_approach,
                    on_combat_event_add_stat_modifier,
                    on_combat_event_add_stat_modifier,
                    on_combat_event_combat_log,
                ),
                (
                    update_attack_timer,
                    update_dodge_timer,
                    update_manual_dodge_timer,
                    update_manual_block_timer,
                    update_block_timer,
                    update_flee_timer,
                    update_condition_timer,
                    health_regen,
                    vigor_regen,
                    update_cooldowns,
                    on_hostile_death,
                    on_player_death,
                ),
            ),
        );
    }
}
