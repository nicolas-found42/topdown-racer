//! Shared overlay screen helpers for the menu and results screens.

use bevy::prelude::*;

use crate::menu::MenuUi;
use crate::results::ResultsUi;

/// Spawns a full-screen overlay root node with a centered column layout.
pub(crate) fn spawn_overlay_root<'a>(
    commands: &'a mut Commands,
    row_gap: f32,
    alpha: f32,
) -> bevy::ecs::system::EntityCommands<'a> {
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(row_gap),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.05, 0.05, 0.08, alpha)),
        ..default()
    })
}

/// Query matching menu and results screen roots.
type ScreensQuery<'w, 's> = Query<'w, 's, Entity, Or<(With<MenuUi>, With<ResultsUi>)>>;

/// Despawns menu and results screens when leaving them.
pub fn despawn_screens(mut commands: Commands, screens_q: ScreensQuery) {
    for entity in screens_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
