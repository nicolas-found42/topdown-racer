use bevy::prelude::*;
use topdown_racer::RacerGamePlugin;

const GAME_TITLE: &str = "Topdown Racer";

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: GAME_TITLE.to_owned(),
                        resize_constraints: WindowResizeConstraints { min_width: 640.0, min_height: 360.0, ..default() },
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::BLACK)) // Black letterbox bars
        .add_plugins(RacerGamePlugin)
        .run();
}
