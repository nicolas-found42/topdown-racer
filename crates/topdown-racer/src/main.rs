use bevy::prelude::*;
use topdown_racer::RacerGamePlugin;

const GAME_TITLE: &str = "Topdown Racer";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: GAME_TITLE.to_owned(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK)) // Black letterbox bars
        .add_plugins(RacerGamePlugin)
        .run();
}
