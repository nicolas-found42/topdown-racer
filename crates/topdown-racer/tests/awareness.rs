use glam::Vec2;
use topdown_racer::awareness::{rival_indicator, CameraLead, CameraMode};

#[test]
fn lead_is_bounded_and_nearby_indicators_respect_the_viewport() {
    let mut lead = CameraLead::default();
    assert_eq!(lead.step(Vec2::ZERO, CameraMode::LookAhead), Vec2::ZERO);
    for _ in 0..400 {
        lead.step(Vec2::new(0.0, 30.0), CameraMode::LookAhead);
    }
    let offset = lead.step(Vec2::new(0.0, 30.0), CameraMode::LookAhead);
    assert!(offset.y > 9.0 && offset.length() <= 10.001);
    for _ in 0..400 {
        assert!(
            lead.step(Vec2::new(0.0, -30.0), CameraMode::LookAhead)
                .length()
                <= 10.001
        );
    }
    assert_eq!(lead.step(Vec2::X, CameraMode::Centered), Vec2::ZERO);
    assert_eq!(
        rival_indicator(Vec2::ZERO, Vec2::new(20.0, 0.0), Vec2::ZERO),
        None
    );
    assert_eq!(
        rival_indicator(Vec2::ZERO, Vec2::new(80.0, 0.0), Vec2::ZERO),
        None
    );
    let marker = rival_indicator(Vec2::ZERO, Vec2::new(50.0, 0.0), Vec2::ZERO).unwrap();
    assert_eq!(marker, Vec2::new(38.0, 0.0));
}
