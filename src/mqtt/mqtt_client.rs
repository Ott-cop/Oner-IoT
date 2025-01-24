use std::time::Duration;

use esp_idf_svc::{mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration, MqttProtocolVersion}, sys::EspError};

pub fn mqtt_create<'a>(
    url: &str,
    client_id: &str,
    client_pass: &str
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            username: Some(client_id),
            password: Some(client_pass),
            reconnect_timeout: Some(Duration::from_millis(2000)),
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            server_certificate: None,
            client_certificate: None,
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}
