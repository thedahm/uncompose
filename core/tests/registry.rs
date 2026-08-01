//! The static registry: preset -> model resolution and license/tier facts.

use uncompose_core::registry;

#[test]
fn presets_resolve_to_their_pipeline_models_in_order() {
    let six = registry::resolve("6-stem").expect("6-stem is a preset");
    let ids: Vec<&str> = six.iter().map(|m| m.id).collect();
    assert_eq!(ids, ["mel_band_roformer_kim", "htdemucs_6s"]);

    let two = registry::resolve("2-stem").expect("2-stem is a preset");
    let ids: Vec<&str> = two.iter().map(|m| m.id).collect();
    assert_eq!(ids, ["mel_band_roformer_kim"]);
}

#[test]
fn a_bare_model_id_resolves_to_just_that_model() {
    let one = registry::resolve("htdemucs_6s").expect("known model");
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].id, "htdemucs_6s");
}

#[test]
fn an_unknown_target_resolves_to_nothing() {
    assert!(registry::resolve("nope").is_none());
}

#[test]
fn every_model_relays_a_license_and_a_hardware_tier() {
    for entry in registry::REGISTRY {
        assert!(!entry.license.is_empty(), "{} has no license", entry.id);
        // label() is total; this just asserts the field is wired.
        let _ = entry.hardware_tier.label();
    }
}
