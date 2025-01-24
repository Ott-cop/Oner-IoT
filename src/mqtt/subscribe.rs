use std::{sync::{Arc, Mutex}, thread, time::Duration};
use esp_idf_svc::mqtt::client::EspMqttClient;
use log::info;
use esp_idf_svc::mqtt::client::QoS;

use crate::{TOPIC_PIN_2, TOPIC_PIN_23};

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

pub fn subscribe(
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