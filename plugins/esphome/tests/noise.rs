use esphome_client::{
    EspHomeClient,
    types::{
        ButtonCommandRequest, ClimateCommandRequest, CoverCommandRequest, EspHomeMessage,
        FanCommandRequest, LightCommandRequest, ListEntitiesRequest, SubscribeStatesRequest,
        SwitchCommandRequest,
    },
};
use esphome_native_api::{
    esphomeapi::EspHomeApi,
    parser::ProtoMessage,
    proto::{
        BinarySensorStateResponse, ClimateStateResponse, CoverStateResponse, FanStateResponse,
        LightStateResponse, ListEntitiesBinarySensorResponse, ListEntitiesButtonResponse,
        ListEntitiesClimateResponse, ListEntitiesCoverResponse, ListEntitiesDoneResponse,
        ListEntitiesFanResponse, ListEntitiesLightResponse, ListEntitiesNumberResponse,
        ListEntitiesSensorResponse, ListEntitiesSwitchResponse, ListEntitiesTextSensorResponse,
        SensorStateResponse, SwitchStateResponse,
    },
};
use std::collections::BTreeSet;
use tokio::{net::TcpListener, sync::mpsc, time::Duration};

const FIXTURE_KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

#[derive(Debug, Clone, PartialEq)]
enum ObservedCommand {
    Switch(bool),
    LightOn(bool),
    LightLevel(f32),
    LightRgb(f32, f32, f32),
    FanOn(bool),
    FanLevel(i32),
    ClimateMode(i32),
    ClimateTargets(f32, f32),
    ClimateFan(i32),
    ClimateSwing(i32),
    ClimatePreset(i32),
    CoverLift(f32),
    CoverTilt(f32),
    CoverStop,
    Button,
}

#[tokio::test]
#[allow(deprecated, reason = "API 1.14 fixture populates retained wire fields")]
async fn pinned_noise_api_1_14_covers_mapped_families_feature_gates_and_switch_restore() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (command_tx, mut command_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let api = EspHomeApi::builder()
            .name("fixture".into())
            .encryption_key(FIXTURE_KEY.into())
            .api_version_major(1)
            .api_version_minor(14)
            .build();
        let (tx, mut rx) = api.start(stream).await.unwrap();
        while let Ok(message) = rx.recv().await {
            match message {
                ProtoMessage::ListEntitiesRequest(_) => {
                    for entity in fixture_entities() {
                        tx.send(entity).await.unwrap();
                    }
                    tx.send(ProtoMessage::ListEntitiesDoneResponse(
                        ListEntitiesDoneResponse {},
                    ))
                    .await
                    .unwrap();
                }
                ProtoMessage::SubscribeStatesRequest(_) => {
                    for state in fixture_states() {
                        tx.send(state).await.unwrap();
                    }
                }
                ProtoMessage::SwitchCommandRequest(command) => {
                    command_tx
                        .send(ObservedCommand::Switch(command.state))
                        .unwrap();
                    tx.send(ProtoMessage::SwitchStateResponse(SwitchStateResponse {
                        key: command.key,
                        state: command.state,
                        device_id: command.device_id,
                    }))
                    .await
                    .unwrap();
                }
                ProtoMessage::LightCommandRequest(command) => {
                    let observed = if command.has_state {
                        ObservedCommand::LightOn(command.state)
                    } else if command.has_brightness {
                        ObservedCommand::LightLevel(command.brightness)
                    } else {
                        ObservedCommand::LightRgb(command.red, command.green, command.blue)
                    };
                    command_tx.send(observed).unwrap();
                }
                ProtoMessage::FanCommandRequest(command) => {
                    command_tx
                        .send(if command.has_state {
                            ObservedCommand::FanOn(command.state)
                        } else {
                            ObservedCommand::FanLevel(command.speed_level)
                        })
                        .unwrap();
                }
                ProtoMessage::ClimateCommandRequest(command) => {
                    let observed = if command.has_mode {
                        ObservedCommand::ClimateMode(command.mode)
                    } else if command.has_target_temperature_low
                        && command.has_target_temperature_high
                    {
                        ObservedCommand::ClimateTargets(
                            command.target_temperature_low,
                            command.target_temperature_high,
                        )
                    } else if command.has_fan_mode {
                        ObservedCommand::ClimateFan(command.fan_mode)
                    } else if command.has_swing_mode {
                        ObservedCommand::ClimateSwing(command.swing_mode)
                    } else {
                        ObservedCommand::ClimatePreset(command.preset)
                    };
                    command_tx.send(observed).unwrap();
                }
                ProtoMessage::CoverCommandRequest(command) => {
                    let observed = if command.stop {
                        ObservedCommand::CoverStop
                    } else if command.has_position {
                        ObservedCommand::CoverLift(command.position)
                    } else {
                        ObservedCommand::CoverTilt(command.tilt)
                    };
                    command_tx.send(observed).unwrap();
                }
                ProtoMessage::ButtonCommandRequest(_) => {
                    command_tx.send(ObservedCommand::Button).unwrap();
                }
                _ => {}
            }
        }
    });

    let mut client = EspHomeClient::builder()
        .address(&address.to_string())
        .key(FIXTURE_KEY)
        .client_info("litehouse-esphome-fixture")
        .timeout(Duration::from_secs(2))
        .connect()
        .await
        .unwrap();
    client.try_write(ListEntitiesRequest {}).await.unwrap();
    client.try_write(SubscribeStatesRequest {}).await.unwrap();

    let mut listed = BTreeSet::new();
    let mut states = BTreeSet::new();
    let mut listing_done = false;
    while !listing_done || listed.len() < 10 || states.len() < 7 {
        match tokio::time::timeout(Duration::from_secs(2), client.try_read())
            .await
            .unwrap()
            .unwrap()
        {
            EspHomeMessage::ListEntitiesSwitchResponse(entity) => {
                assert_eq!((entity.device_id, entity.key), (1, 11));
                listed.insert("switch");
            }
            EspHomeMessage::ListEntitiesBinarySensorResponse(entity) => {
                assert_eq!(entity.device_class, "door");
                listed.insert("binary");
            }
            EspHomeMessage::ListEntitiesSensorResponse(entity) if entity.key == 31 => {
                assert_eq!(
                    (
                        entity.device_class.as_str(),
                        entity.unit_of_measurement.as_str()
                    ),
                    ("temperature", "°C")
                );
                listed.insert("sensor");
            }
            EspHomeMessage::ListEntitiesSensorResponse(entity) => {
                assert_eq!(entity.unit_of_measurement, "furlong");
                listed.insert("unsupported-unit");
            }
            EspHomeMessage::ListEntitiesLightResponse(entity) => {
                assert!(entity.supported_color_modes.contains(&3));
                assert!(entity.supported_color_modes.contains(&35));
                assert!(entity.supported_color_modes.contains(&11));
                listed.insert("light");
            }
            EspHomeMessage::ListEntitiesFanResponse(entity) => {
                assert!(entity.supports_speed);
                assert_eq!(entity.supported_speed_count, 3);
                listed.insert("fan");
            }
            EspHomeMessage::ListEntitiesClimateResponse(entity) => {
                assert!(entity.supports_two_point_target_temperature);
                assert_eq!(entity.supported_modes, [0, 1, 2, 3]);
                assert_eq!(entity.supported_fan_modes, [2, 5]);
                assert_eq!(entity.supported_swing_modes, [0, 2]);
                assert_eq!(entity.supported_presets, [0, 5]);
                listed.insert("climate");
            }
            EspHomeMessage::ListEntitiesCoverResponse(entity) => {
                assert!(entity.supports_position && entity.supports_tilt && entity.supports_stop);
                listed.insert("cover");
            }
            EspHomeMessage::ListEntitiesButtonResponse(_) => {
                listed.insert("button");
            }
            EspHomeMessage::ListEntitiesTextSensorResponse(_) => {
                listed.insert("unsupported-text");
            }
            EspHomeMessage::ListEntitiesNumberResponse(_) => {
                listed.insert("unsupported-number");
            }
            EspHomeMessage::ListEntitiesDoneResponse(_) => listing_done = true,
            EspHomeMessage::SwitchStateResponse(state) => {
                assert!(!state.state);
                states.insert("switch");
            }
            EspHomeMessage::BinarySensorStateResponse(state) => {
                assert!(state.state && !state.missing_state);
                states.insert("binary");
            }
            EspHomeMessage::SensorStateResponse(state) => {
                assert_eq!(state.state, 21.5);
                states.insert("sensor");
            }
            EspHomeMessage::LightStateResponse(state) => {
                assert!(state.state && state.brightness > 0.0);
                states.insert("light");
            }
            EspHomeMessage::FanStateResponse(state) => {
                assert!(state.state && state.speed_level == 2);
                states.insert("fan");
            }
            EspHomeMessage::ClimateStateResponse(state) => {
                assert_eq!(
                    (state.target_temperature_low, state.target_temperature_high),
                    (18.0, 24.0)
                );
                states.insert("climate");
            }
            EspHomeMessage::CoverStateResponse(state) => {
                assert_eq!((state.position, state.tilt), (0.25, 0.5));
                states.insert("cover");
            }
            _ => {}
        }
    }

    // Preserve the original absolute set/observe/restore regression.
    client
        .try_write(SwitchCommandRequest {
            key: 11,
            state: true,
            device_id: 1,
        })
        .await
        .unwrap();
    observe_switch(&mut client, true).await;
    client
        .try_write(SwitchCommandRequest {
            key: 11,
            state: false,
            device_id: 1,
        })
        .await
        .unwrap();
    observe_switch(&mut client, false).await;

    client
        .try_write(LightCommandRequest {
            key: 41,
            has_state: true,
            state: false,
            device_id: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .try_write(LightCommandRequest {
            key: 41,
            has_brightness: true,
            brightness: 0.5,
            device_id: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .try_write(LightCommandRequest {
            key: 41,
            has_color_mode: true,
            color_mode: 35,
            has_rgb: true,
            red: 0.1,
            green: 0.2,
            blue: 0.3,
            device_id: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .try_write(FanCommandRequest {
            key: 51,
            has_state: true,
            state: false,
            device_id: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .try_write(FanCommandRequest {
            key: 51,
            has_speed_level: true,
            speed_level: 3,
            device_id: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    for command in [
        ClimateCommandRequest {
            key: 71,
            has_mode: true,
            mode: 2,
            device_id: 1,
            ..Default::default()
        },
        ClimateCommandRequest {
            key: 71,
            has_target_temperature_low: true,
            target_temperature_low: 19.0,
            has_target_temperature_high: true,
            target_temperature_high: 23.0,
            device_id: 1,
            ..Default::default()
        },
        ClimateCommandRequest {
            key: 71,
            has_fan_mode: true,
            fan_mode: 5,
            device_id: 1,
            ..Default::default()
        },
        ClimateCommandRequest {
            key: 71,
            has_swing_mode: true,
            swing_mode: 2,
            device_id: 1,
            ..Default::default()
        },
        ClimateCommandRequest {
            key: 71,
            has_preset: true,
            preset: 5,
            device_id: 1,
            ..Default::default()
        },
    ] {
        client.try_write(command).await.unwrap();
    }
    for command in [
        CoverCommandRequest {
            key: 81,
            has_position: true,
            position: 0.75,
            device_id: 1,
            ..Default::default()
        },
        CoverCommandRequest {
            key: 81,
            has_tilt: true,
            tilt: 0.25,
            device_id: 1,
            ..Default::default()
        },
        CoverCommandRequest {
            key: 81,
            stop: true,
            device_id: 1,
            ..Default::default()
        },
    ] {
        client.try_write(command).await.unwrap();
    }
    client
        .try_write(ButtonCommandRequest {
            key: 91,
            device_id: 1,
        })
        .await
        .unwrap();

    let expected = vec![
        ObservedCommand::Switch(true),
        ObservedCommand::Switch(false),
        ObservedCommand::LightOn(false),
        ObservedCommand::LightLevel(0.5),
        ObservedCommand::LightRgb(0.1, 0.2, 0.3),
        ObservedCommand::FanOn(false),
        ObservedCommand::FanLevel(3),
        ObservedCommand::ClimateMode(2),
        ObservedCommand::ClimateTargets(19.0, 23.0),
        ObservedCommand::ClimateFan(5),
        ObservedCommand::ClimateSwing(2),
        ObservedCommand::ClimatePreset(5),
        ObservedCommand::CoverLift(0.75),
        ObservedCommand::CoverTilt(0.25),
        ObservedCommand::CoverStop,
        ObservedCommand::Button,
    ];
    let mut observed = Vec::new();
    while observed.len() < expected.len() {
        observed.push(
            tokio::time::timeout(Duration::from_secs(2), command_rx.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    assert_eq!(observed, expected);

    drop(client);
    server.abort();
}

async fn observe_switch(client: &mut EspHomeClient, expected: bool) {
    loop {
        if let EspHomeMessage::SwitchStateResponse(state) =
            tokio::time::timeout(Duration::from_secs(2), client.try_read())
                .await
                .unwrap()
                .unwrap()
            && state.key == 11
            && state.state == expected
        {
            return;
        }
    }
}

#[allow(deprecated, reason = "API 1.14 fixture populates retained wire fields")]
fn fixture_entities() -> Vec<ProtoMessage> {
    vec![
        ProtoMessage::ListEntitiesSwitchResponse(ListEntitiesSwitchResponse {
            object_id: "relay_1".into(),
            key: 11,
            name: "Relay 1".into(),
            device_class: "switch".into(),
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesBinarySensorResponse(ListEntitiesBinarySensorResponse {
            object_id: "door".into(),
            key: 21,
            name: "Door".into(),
            device_class: "door".into(),
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesSensorResponse(ListEntitiesSensorResponse {
            object_id: "temperature".into(),
            key: 31,
            name: "Temperature".into(),
            unit_of_measurement: "°C".into(),
            accuracy_decimals: 1,
            device_class: "temperature".into(),
            state_class: 1,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesSensorResponse(ListEntitiesSensorResponse {
            object_id: "unsupported".into(),
            key: 32,
            name: "Unsupported".into(),
            unit_of_measurement: "furlong".into(),
            device_class: "distance".into(),
            state_class: 1,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesLightResponse(ListEntitiesLightResponse {
            object_id: "lamp".into(),
            key: 41,
            name: "Lamp".into(),
            supported_color_modes: vec![3, 35, 11],
            min_mireds: 153.0,
            max_mireds: 500.0,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesFanResponse(ListEntitiesFanResponse {
            object_id: "fan".into(),
            key: 51,
            name: "Fan".into(),
            supports_speed: true,
            supported_speed_count: 3,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesClimateResponse(ListEntitiesClimateResponse {
            object_id: "hvac".into(),
            key: 71,
            name: "HVAC".into(),
            supports_current_temperature: true,
            supports_two_point_target_temperature: true,
            supported_modes: vec![0, 1, 2, 3],
            visual_min_temperature: 10.0,
            visual_max_temperature: 30.0,
            visual_target_temperature_step: 0.5,
            supports_action: true,
            supported_fan_modes: vec![2, 5],
            supported_swing_modes: vec![0, 2],
            supported_presets: vec![0, 5],
            temperature_unit: 0,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesCoverResponse(ListEntitiesCoverResponse {
            object_id: "blind".into(),
            key: 81,
            name: "Blind".into(),
            supports_position: true,
            supports_tilt: true,
            supports_stop: true,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesButtonResponse(ListEntitiesButtonResponse {
            object_id: "identify".into(),
            key: 91,
            name: "Identify".into(),
            device_class: "identify".into(),
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesTextSensorResponse(ListEntitiesTextSensorResponse {
            object_id: "version".into(),
            key: 101,
            name: "Version".into(),
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ListEntitiesNumberResponse(ListEntitiesNumberResponse {
            object_id: "threshold".into(),
            key: 102,
            name: "Threshold".into(),
            device_id: 1,
            ..Default::default()
        }),
    ]
}

#[allow(deprecated, reason = "API 1.14 fixture populates retained wire fields")]
fn fixture_states() -> Vec<ProtoMessage> {
    vec![
        ProtoMessage::SwitchStateResponse(SwitchStateResponse {
            key: 11,
            state: false,
            device_id: 1,
        }),
        ProtoMessage::BinarySensorStateResponse(BinarySensorStateResponse {
            key: 21,
            state: true,
            missing_state: false,
            device_id: 1,
        }),
        ProtoMessage::SensorStateResponse(SensorStateResponse {
            key: 31,
            state: 21.5,
            missing_state: false,
            device_id: 1,
        }),
        ProtoMessage::LightStateResponse(LightStateResponse {
            key: 41,
            state: true,
            brightness: 0.75,
            color_mode: 35,
            red: 0.1,
            green: 0.2,
            blue: 0.3,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::FanStateResponse(FanStateResponse {
            key: 51,
            state: true,
            speed_level: 2,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::ClimateStateResponse(ClimateStateResponse {
            key: 71,
            mode: 1,
            current_temperature: 21.0,
            target_temperature_low: 18.0,
            target_temperature_high: 24.0,
            action: 4,
            fan_mode: 2,
            swing_mode: 0,
            preset: 0,
            device_id: 1,
            ..Default::default()
        }),
        ProtoMessage::CoverStateResponse(CoverStateResponse {
            key: 81,
            position: 0.25,
            tilt: 0.5,
            current_operation: 0,
            device_id: 1,
            ..Default::default()
        }),
    ]
}
