// SPDX-License-Identifier: LicenseRef-Blockscout

use blockscout_service_launcher::launcher::ConfigSettings;
use pretty_assertions::assert_eq;
use sig_provider_server::Settings;

#[test]
fn base_settings_are_default() {
    std::env::set_var("SIG_PROVIDER__CONFIG", "config/base.toml");
    let example = Settings::build().expect("Failed to parse config");
    let default = Settings::default();
    assert_eq!(default, example);
}

#[test]
fn base_settings_include_all() {
    let default = Settings::default();
    let default = toml::to_string(&default).unwrap();
    let default: toml::Value = toml::from_str(&default).unwrap();

    let example = std::fs::read("config/base.toml").unwrap();
    let example: toml::Value = toml::from_slice(&example).unwrap();
    assert_eq!(default, example);
}

#[test]
fn openchain_is_accepted_as_sigeth() {
    let settings: Settings = toml::from_str(
        r#"
        [sources]
        openchain = "https://example.com/"
        "#,
    )
    .expect("Failed to parse config");
    assert_eq!("https://example.com/", settings.sources.sigeth.as_str());
}

#[test]
fn openchain_and_sigeth_are_not_accepted_together() {
    let result = toml::from_str::<Settings>(
        r#"
        [sources]
        openchain = "https://example.com/"
        sigeth = "https://example.org/"
        "#,
    );
    assert!(result.is_err(), "expected an error, got {result:?}");
}
