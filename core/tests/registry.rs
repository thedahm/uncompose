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
fn every_weight_file_pins_a_real_sha256() {
    for entry in registry::MANIFEST {
        for file in entry.files {
            assert_eq!(
                file.sha256.len(),
                64,
                "{}: {} has no pinned digest",
                entry.id,
                file.file_name
            );
            assert!(
                file.sha256.chars().all(|c| c.is_ascii_hexdigit()),
                "{}: {} digest is not hex",
                entry.id,
                file.file_name
            );
        }
    }
}

#[test]
fn manifest_files_are_the_ones_the_engine_loads() {
    // The engine asks audio-separator for these exact filenames; the manifest
    // must cache weights under the same names or the engine re-downloads.
    let htdemucs = registry::find("htdemucs_6s").expect("known model");
    let names: Vec<&str> = htdemucs.files.iter().map(|f| f.file_name).collect();
    assert!(names.contains(&"htdemucs_6s.yaml"), "got {names:?}");
    assert!(names.contains(&"5c90dfd2-34c22ccb.th"), "got {names:?}");

    let roformer = registry::find("mel_band_roformer_kim").expect("known model");
    let names: Vec<&str> = roformer.files.iter().map(|f| f.file_name).collect();
    assert!(
        names.contains(&"vocals_mel_band_roformer.ckpt"),
        "got {names:?}"
    );
    assert!(
        names.contains(&"vocals_mel_band_roformer.yaml"),
        "got {names:?}"
    );
}

#[test]
fn every_model_relays_a_license_and_a_hardware_tier() {
    for entry in registry::MANIFEST {
        assert!(
            !entry.license.label.is_empty(),
            "{} has no license",
            entry.id
        );
        // label() is total; this just asserts the field is wired.
        let _ = entry.hardware_tier.label();
    }
}
