use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::{EspCustomNvsPartition, EspDefaultNvsPartition, EspNvs};
use esp_idf_svc::sys::esp_task_wdt_deinit;
use log::info;
use mqtt::mqtt_client::mqtt_create;
use mqtt::subscribe::subscribe;
use serde_json::Value;
use storage::storage::Storage;
use wifi::wifi::wifi_create;
use mqtt::mqtt_runner::{message_distributor, runner};
use device::{call_device_state, Device};
use serde::{Serialize, Deserialize};
pub mod device;
pub mod wifi;
pub mod mqtt;
pub mod storage;

const ENVCONFIGURATION: &str = include_str!("../env.json");

const TOPIC_PIN_2: &str = "pin_2";
const TOPIC_PIN_23: &str = "pin_23";

const NAMESPACE: &str = "devices_data";
const DEVICES_TAG: &str = "devices_tag";

devices_layout!(pin_2, pin_23);

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
    let nvs_data_partition = EspCustomNvsPartition::take("nvs_data").expect("Problem to use nvs_data partition! Restarting...");
    
    let nvs_data = Arc::new(Mutex::new(Storage::new(EspNvs::new(nvs_data_partition, NAMESPACE, true).expect("Problem to create a NVS Instance..."))));
    let nvs_data_clone1 = nvs_data.clone();
    let nvs_data_clone2 = nvs_data.clone();

    let pin_driver_2 = Arc::new(Mutex::new(PinDriver::input_output(peripherals.pins.gpio2).unwrap()));
    let pd_2 = pin_driver_2.clone();

    let pin_driver_23 = Arc::new(Mutex::new(PinDriver::input_output(peripherals.pins.gpio23).unwrap()));
    let pd_23 = pin_driver_23.clone();

    {
        let mut nvs_data = nvs_data.lock().unwrap();
        let try_get = nvs_data.get(DEVICES_TAG);

        if let Err(err) = try_get {
            panic!("[ERROR] {err}");
        } 
        else if let Ok(Some(data)) = try_get {
            info!("Found data from NVS!");
            
            let devices_json = serde_json::from_str::<LayoutDataDevice>(data.as_str()).expect("Impossible to parse data");

            call_device_state(pd_2, devices_json.pin_2.state);
            call_device_state(pd_23, devices_json.pin_23.state);
        }
        else {
            info!("Not found data. Creating a layout data...");

            if let Err(err) = nvs_data.set_default(DEVICES_TAG) {
                panic!("[ERROR] {err}");
            }
            else {
                info!("Created data layout!");
            }
        }
    }
    

    let _wifi = wifi_create(&sys_loop, &nvs, wifi_ssid, wifi_pass).unwrap();

    let (client, mut conn) = mqtt_create(mqtt_url, mqtt_client_id, mqtt_client_pass).unwrap();
 
    let client = Arc::new(Mutex::new(client));
    let c1 = client.clone();

    let (mut tx1, rx1) = channel::<String>();
    let (mut tx2, rx2) = channel::<String>();

    subscribe(c1);
    
    runner(rx1, nvs_data_clone1, TOPIC_PIN_2, pin_driver_2);
    runner(rx2, nvs_data_clone2, TOPIC_PIN_23, pin_driver_23);
   
    message_distributor((&mut tx1, TOPIC_PIN_2), (&mut tx2, TOPIC_PIN_23), &mut conn);
    
    loop {}
}
