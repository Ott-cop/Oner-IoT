use std::{sync::{mpsc::{Receiver, Sender}, Arc, Mutex}, thread};

use esp_idf_svc::{hal::gpio::{InputOutput, Pin, PinDriver}, mqtt::client::{EspMqttConnection, EventPayload}};
use log::{info, Level};
use serde_json::Value;

use crate::{device::Device, storage::storage::Storage, LayoutDataDevice, DEVICES_TAG, TOPIC_PIN_2, TOPIC_PIN_23};

pub fn message_distributor(
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

pub fn runner<'a, P: Pin>(
    rx: Receiver<String>,
    nvs_data: Arc<Mutex<Storage>>,
    pin_str: &'static str,
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
                {
                    let mut pin_driver = pin_driver.lock().unwrap();
                    let mut nvs_data = nvs_data.lock().unwrap();

                    if device.state == true {
                        if let Ok(Some(raw_json)) = nvs_data.get(DEVICES_TAG) {
                            let mut devices_json = serde_json::from_str::<Value>(raw_json.as_str()).unwrap();
                            
                            {
                                let dev = devices_json.get_mut(pin_str).unwrap();
                                let mut device = serde_json::from_value::<Device>(dev.clone()).unwrap();
                                device.state = true;
                                *dev = serde_json::to_value(device).unwrap();
                            }
        
                            if let Err(err) = nvs_data.set(DEVICES_TAG, serde_json::from_value::<LayoutDataDevice>(devices_json.clone()).unwrap()) {
                                log::log!(Level::Error, "{err}");
                                continue;
                            }
                            
                            _ = pin_driver.set_high();
                        } 
                    } 
                    
                    else {
                        if let Ok(Some(raw_json)) = nvs_data.get(DEVICES_TAG) {
                            let mut devices_json = serde_json::from_str::<Value>(raw_json.as_str()).unwrap();

                            {
                                let dev = devices_json.get_mut(pin_str).unwrap();
                                let mut device = serde_json::from_value::<Device>(dev.clone()).unwrap();
                                device.state = true;
                                *dev = serde_json::to_value(device).unwrap();
                            }
        
                            if let Err(err) = nvs_data.set(DEVICES_TAG, serde_json::from_value::<LayoutDataDevice>(devices_json.clone()).unwrap()) {
                                log::log!(Level::Error, "{err}");
                                continue;
                            }
                        
                            _ = pin_driver.set_low();
                        }
                    }
                }
     
            } 
        }
    });
}
