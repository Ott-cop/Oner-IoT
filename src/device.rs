use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::gpio::{InputOutput, Pin, PinDriver};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: i32,
    pub name: String,
    pub state: bool
}

pub fn call_device_state<P: Pin>(
    pin_driver: Arc<Mutex<PinDriver<'static, P, InputOutput>>>,
    nvs_device_state: bool
) {
    let mut pin_driver = pin_driver.lock().unwrap();

    if nvs_device_state {
        _ = pin_driver.set_high();
    } else {
        _ = pin_driver.set_low();
    }
}


#[macro_export]
macro_rules! devices_layout {
    ( $( $pin:ident ),+ ) => {
        
        #[derive(Debug, Serialize, Deserialize)]
        pub struct LayoutDataDevice {
            $(
                $pin: Device,
            )+
        }
        
        impl LayoutDataDevice {
            pub fn new() -> Self {
                LayoutDataDevice {
                    $(
                        $pin: Device {
                            id: 0,
                            name: String::from(""),
                            state: false
                        },
                    )+
                }
            }
        }
    
    };    
}
