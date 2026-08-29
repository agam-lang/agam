//! Golden snapshot test for 2D scene graph serialization.
//!
//! Asserts that the retained scene node tree serializes deterministically
//! to JSON and matches the checked-in golden snapshot fixture.

use agam_gui::{Color, Rect, SceneBuilder};

fn build_standard_3rect_clip_scene() -> SceneBuilder {
    let mut builder = SceneBuilder::new();

    // 1. Background rect
    builder.fill_rect(Rect::new(0.0, 0.0, 800.0, 600.0), Color::DARK_GRAY);

    // 2. Push rounded clip
    builder.push_clip_rounded_rect(Rect::new(50.0, 50.0, 300.0, 200.0), 16.0);

    // 3. Card surface rect inside clip
    builder.fill_rect(Rect::new(50.0, 50.0, 300.0, 200.0), Color::rgb(45, 45, 45));

    // 4. Accent button rect inside clip
    builder.fill_rounded_rect(Rect::new(70.0, 70.0, 120.0, 40.0), 8.0, Color::BLUE);

    // 5. Pop clip
    builder.pop_clip();

    builder
}

#[test]
fn test_golden_scene() {
    let scene = build_standard_3rect_clip_scene();
    let actual_json = scene
        .to_json_pretty()
        .expect("Scene must serialize to JSON");

    let golden_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/scene_3rect_clip.json"
    );

    let golden_json = std::fs::read_to_string(golden_path)
        .expect("Golden snapshot file must exist at tests/golden/scene_3rect_clip.json");

    // Normalize line endings for cross-platform deterministic comparison
    let normalized_actual = actual_json.replace("\r\n", "\n").trim().to_string();
    let normalized_golden = golden_json.replace("\r\n", "\n").trim().to_string();

    assert_eq!(
        normalized_actual, normalized_golden,
        "Serialized scene graph diverged from golden snapshot!"
    );

    // Round-trip deserialization verification
    let deserialized = SceneBuilder::from_json(&normalized_actual)
        .expect("Scene must deserialize cleanly from JSON snapshot");
    assert_eq!(scene, deserialized);
}
