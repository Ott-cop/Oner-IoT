use device::Device;
use embassy_futures::select::{select, Either};
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::{gpio::{InputOutput, InputPin, OutputPin, Pin, PinDriver}, prelude::Peripherals}, mqtt::client::{EspAsyncMqttClient, EspAsyncMqttConnection, EventPayload, MessageId, MqttClientConfiguration, QoS}, nvs::EspDefaultNvsPartition, sys::EspError, timer::{EspAsyncTimer, EspTaskTimerService, EspTimerService}, wifi::{AsyncWifi, ClientConfiguration, EspWifi}};
use log::{info, error};
use serde::{Deserialize, Serialize};
use core::str;
use std::{ pin::pin, sync::{Arc, RwLock}, time::Duration};

pub mod device;

const WIFI_SSID: &str = "TP-Link_DF5D";
const WIFI_PASS: &str = "4444jhin";

const MQTT_URL: &str = "mqtt://192.168.0.100:1883";
const MQTT_CLIENT_ID: &str = "esp32";
const MQTT_CLIENT_PASSWORD: &str = "12342234";

#[derive(Deserialize, Serialize)]
struct MsgReceived<'a> {
    id: MessageId,
    topic: Option<&'a str>,
    data: &'a [u8]
}


fn main() {

    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    unsafe {
        esp_idf_svc::sys::esp_task_wdt_deinit();
        esp_idf_svc::sys::esp_wifi_set_ps(esp_idf_svc::sys::wifi_ps_type_t_WIFI_PS_NONE);
    }

    let sysloop = EspSystemEventLoop::take().unwrap();
    let timer_service = EspTimerService::new().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    let peripherals = unsafe { Peripherals::new() };

    let pin23 = io_pin(peripherals.pins.gpio23);

    esp_idf_svc::hal::task::block_on(async {
        let _wifi_create = wifi_create(&sysloop, &timer_service, &nvs).await.unwrap();
        info!("Wifi Created!");

        let (mut client, mut conn) = mqtt_create(MQTT_URL, MQTT_CLIENT_ID, MQTT_CLIENT_PASSWORD).unwrap();
        info!("MQTT Client Created!");

        let mut timer = timer_service.timer_async().unwrap();
        run(Arc::clone(&pin23), &mut client, &mut conn, &mut timer, "changeState").await
    }).unwrap();
    
}

fn io_pin<'a, P: Pin + OutputPin + InputPin>(pin: P) ->  Arc<RwLock<PinDriver<'a, P, InputOutput>>> {
    Arc::new(RwLock::new(PinDriver::input_output(pin).unwrap())) 
}

async fn run<T: InputPin + OutputPin>(
    pin: Arc<RwLock<PinDriver<'_, T, InputOutput>>>,
    client: &mut EspAsyncMqttClient,
    connection: &mut EspAsyncMqttConnection,
    timer: &mut EspAsyncTimer,
    topic: &str,
) -> Result<(), EspError> {
    info!("About to start the MQTT client");
    let pin_reference = pin.clone();
    let pin_reference2 = pin.clone();
    
    let res = select(
        pin!(async move {
            info!("MQTT Listening for messages");

            while let Ok(event) = connection.next().await{
                
                if let EventPayload::Received{ id, topic, data, details } = event.payload() {
                    let mut pin = pin_reference.write().unwrap();
                    let raw_json = str::from_utf8(data).unwrap();
                    info!("ID > {}", id);
                    info!("Topico > {:#?}", topic);
                    info!("Detalhes > {:#?}", details);

                    if let Ok(device) = serde_json::from_str::<Device>(raw_json) {
                        println!("{}", device.state);
                        
                        if topic == Some("changeState") {
                            if device.state == true {
                                (*pin).set_high().unwrap(); 
                            } else {
                                (*pin).set_low().unwrap(); 
                            }
                            
                        }
                    }
                }
            }

            info!("Connection closed");

            Ok(())
        }),
        pin!(async move {
            loop {
                if let Err(e) = client.subscribe(topic, QoS::AtMostOnce).await {
                    error!("Failed to subscribe to topic \"{topic}\": {e}, retrying...");

                    timer.after(Duration::from_millis(500)).await?;

                    continue;
                }

                info!("Subscribed to topic \"{topic}\"");

                timer.after(Duration::from_millis(500)).await?; 

                loop {
                    
                    let pin2 = pin_reference2.read().unwrap();
                    let payload = Device {
                        id: 0,
                        pin: (*pin2).pin().clone(),
                        state: (*pin2).is_high().clone()
                    };
                    drop(pin2);

                    client
                        .publish(topic, QoS::AtMostOnce, false, serde_json::to_string(&payload).unwrap().as_bytes())
                        .await?;

                    info!("Published \"{payload:#?}\" to topic \"{topic}\"");

                    let sleep_secs = 2;

                    info!("Now sleeping for {sleep_secs}s...");
                    timer.after(Duration::from_secs(sleep_secs)).await?;
                }
            }
        }),
    )
    .await;

    match res {
        Either::First(res) => res,
        Either::Second(res) => res,
    }
}

fn mqtt_create(
    url: &str,
    client_id: &str,
    client_pass: &str
) -> Result<(EspAsyncMqttClient, EspAsyncMqttConnection), EspError> {
    
    let (mqtt_client, mqtt_conn) = EspAsyncMqttClient::new(&url, &MqttClientConfiguration {
        client_id: Some(&client_id),
        username: Some(&client_id),
        password: Some(&client_pass),
        client_certificate: None,
        server_certificate: None,
        ..Default::default()
    }).unwrap();

    println!("{} {}", client_id, client_pass);

    Ok((mqtt_client, mqtt_conn))
}


async fn wifi_create(
    sysloop: &EspSystemEventLoop,
    timer: &EspTaskTimerService,
    nvs: &EspDefaultNvsPartition

) -> Result<EspWifi<'static>, EspError> {
    let peripherals = Peripherals::take().unwrap();

    let mut esp_wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs.clone())).unwrap();

    let mut wifi = AsyncWifi::wrap(&mut esp_wifi, sysloop.clone(), timer.clone()).unwrap();

    wifi.set_configuration(&esp_idf_svc::wifi::Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASS.try_into().unwrap(),
        ..Default::default()
    })).unwrap();

    wifi.start().await.unwrap();
    info!("Starting wifi...");

    wifi.connect().await.unwrap();
    info!("Connecting to {}.", WIFI_SSID);

    wifi.wait_netif_up().await.unwrap();
    info!("Wifi Connected!");

    Ok(esp_wifi)
}

