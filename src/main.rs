use core::time::Duration;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use device::Device;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{InputOutput, Pin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{esp_task_wdt_deinit, EspError};
use esp_idf_svc::wifi::*;

use log::*;
use serde_json::Value;
pub mod device;


const ENVCONFIGURATION: &str = include_str!("../env.json");

const TOPIC_PIN_2: &str = "pin_2";
const TOPIC_PIN_23: &str = "pin_23";

fn main() -> ! {
    unsafe {
        esp_task_wdt_deinit();
    }

	let env_configuration = serde_json::from_str::<Value>(ENVCONFIGURATION).expect("Problem to convert env.json");
 	let wifi_ssid: &str = env_configuration["WIFI_SSID"].as_str().expect("Not found WIFI_SSID enviroment variable!");
    let wifi_pass: &str = env_configuration["WIFI_PASS"].as_str().expect("Not found WIFI_PASS enviroment variable!");
	
	let mqtt_url = env_configuration["MQTT_URL"].as_str().expect("Not found MQTT_URL enviroment variable!");
	let mqtt_client_id = env_configuration["MQTT_CLIENT_ID"].as_str().expect("Not found MQTT_CLIENT_ID enviroment variable!");
	let mqtt_client_pass = env_configuration["MQTT_CLIENT_PASS"].as_str().expect("Not found MQTT_CLIENT_PASS enviroment variable!");
	
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();


    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    let peripherals = unsafe { Peripherals::new() };

    let _wifi = wifi_create(&sys_loop, &nvs, wifi_ssid, wifi_pass).unwrap();

    let (client, mut conn) = mqtt_create(mqtt_url, mqtt_client_id, mqtt_client_pass).unwrap();
 
    let client = Arc::new(Mutex::new(client));
    let c1 = client.clone();

    let pin_driver_2 = Arc::new(Mutex::new(PinDriver::input_output(peripherals.pins.gpio2).unwrap()));
    let pin_driver_23 = Arc::new(Mutex::new(PinDriver::input_output(peripherals.pins.gpio23).unwrap()));

    let (mut tx1, rx1) = channel::<String>();
    let (mut tx2, rx2) = channel::<String>();

    subscribe(c1);
    
    runner(rx1, pin_driver_2);
    runner(rx2, pin_driver_23);
   
    message_distributor((&mut tx1, TOPIC_PIN_2), (&mut tx2, TOPIC_PIN_23), &mut conn);
    
    loop {}
}


macro_rules! setup_subscribe {
    ($client:expr, $( $topic:expr ),*) => {
        $(
            if let Err(err) = $client.subscribe($topic, QoS::AtMostOnce) {
                info!("Problem to connect to topic: {err} - \"{}\"", $topic);
                thread::sleep(Duration::from_millis(2000));
                continue;
            }
        )*
    };
}

fn subscribe(
    client: Arc<Mutex<EspMqttClient<'static>>>
) {
    thread::spawn(move || {
        loop {
            {
                let mut client = client.lock().unwrap();
                info!("Trying to connect to topics");

                setup_subscribe!(client, TOPIC_PIN_2, TOPIC_PIN_23);
            }  
            info!("Subscribed to all topics");
            thread::sleep(Duration::from_millis(5000));
        }
    });
}

fn message_distributor(
    tx: (&mut Sender<String>, &str),
    tx2: (&mut Sender<String>, &str),
    conn: &mut EspMqttConnection
) {
    loop {
        let Ok(event) = conn.next() else {
            continue;
        };
        if let EventPayload::Received { topic, data, .. } = event.payload() {
            let raw_json = core::str::from_utf8(data).unwrap().to_owned();
            info!("Topic Received: {topic:#?}");
            match topic {
                Some(TOPIC_PIN_2) => tx.0.send(raw_json).unwrap(),
                Some(TOPIC_PIN_23) => tx2.0.send(raw_json).unwrap(),
                _ => info!("Cannot find this topic")
            }
        }
    }
}

fn runner<P: Pin>(
    rx: Receiver<String>,
    pin_driver: Arc<Mutex<PinDriver<'static, P, InputOutput>>>
) {
    thread::spawn(move || {
        loop {
            if let Ok(value) = rx.recv() {
                let Ok(device) = serde_json::from_str::<Device>(value.as_str()) else {
                    info!("Impossible to parse data!");
                    continue;
                };
                info!("Device: {device:#?}");
                if device.state == true {
                    let mut pin_driver = pin_driver.lock().unwrap();
                    _ = pin_driver.set_high();
                } else {
                    let mut pin_driver = pin_driver.lock().unwrap();
                    _ = pin_driver.set_low();
                }
            } 
        }
    });

}

fn mqtt_create<'a>(
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
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            server_certificate: None,
            client_certificate: None,
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}

fn wifi_create(
    sys_loop: &EspSystemEventLoop,
    nvs: &EspDefaultNvsPartition,
    wifi_ssid: &str,
    wifi_pass: &str
) -> Result<EspWifi<'static>, EspError> {
    let peripherals = Peripherals::take()?;

    let mut esp_wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs.clone()))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sys_loop.clone())?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: wifi_ssid.try_into().unwrap(),
        password: wifi_pass.try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(esp_wifi)
}
