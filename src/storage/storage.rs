use esp_idf_svc::nvs::{EspNvs, NvsCustom};

use crate::LayoutDataDevice;

pub struct Storage {
    nvs: EspNvs<NvsCustom>
}

impl Storage {
    pub fn new(nvs: EspNvs<NvsCustom>) -> Self {
        Storage {
            nvs
        }
    }
    
    pub fn get(&mut self, name: &str) -> Result<Option<String>, String> {
        let mut buff_data: [u8; 1024] = [0; 1024]; 

        let Ok(get_str) = self.nvs.get_str(name, &mut buff_data) else {
            return Err(String::from("Problem to access the data..."))
        };
        
        if let Some(value) = get_str {
            return Ok(Some(value.to_string()))
        }

        Ok(None)
    }

    pub fn set(&mut self, name: &str, value: LayoutDataDevice) -> Result<(), String> {
        let Ok(str_json) = serde_json::to_string(&value) else {
            return Err(String::from("Impossible to parse Layout Data"))
        };
        
        if let Err(_) = self.nvs.set_str(name, str_json.as_str()) {
            return Err(String::from("Problem to set the data"));
        }
        
        Ok(())
    }

    pub fn set_default(&mut self, name: &str) -> Result<(), String> {
        let Ok(str_json) = serde_json::to_string(&LayoutDataDevice::new()) else {
            return Err(String::from("Impossible to parse Layout Data"))
        };
        
        if let Err(_) = self.nvs.set_str(name, str_json.as_str()) {
            return Err(String::from("Problem to set the data"));
        }
        
        Ok(())
    }
}