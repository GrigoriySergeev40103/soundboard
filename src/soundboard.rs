use std::{collections::HashMap, sync::Arc, path::Path, error::Error};

use livesplit_hotkey::{Hook, Hotkey};
use rodio::OutputStreamHandle;

use crate::Sound;

pub struct Soundboard {
    sound_bindings: HashMap<Hotkey, Arc<Sound>>,
    hook: Hook,
    ostream_handle: OutputStreamHandle,
}

impl Soundboard {
    pub fn new(ostream_handle: OutputStreamHandle) -> Result<Self, livesplit_hotkey::Error> {
        let hook = Hook::new()?;

        Ok(Soundboard {
            sound_bindings: HashMap::<Hotkey, Arc<Sound>>::new(),
            hook,
            ostream_handle,
        })
    }

    pub fn bind_new(&mut self, new_sound: Sound, hotkey: Hotkey,) -> Result<(), livesplit_hotkey::Error> {
        let sound_arc = Arc::new(new_sound);
        let ostream_copy = self.ostream_handle.clone();

        self.sound_bindings.insert(hotkey, Arc::clone(&sound_arc));

        let on_hotkey = move || {
            sound_arc.play(&ostream_copy);
        };

        self.hook.register(hotkey, on_hotkey)?;
        Ok(())
    }

    pub fn unbind(&mut self, hotkey: Hotkey) -> Result<(), Box<dyn Error>> {
        if self.sound_bindings.contains_key(&hotkey) {
            self.sound_bindings.remove(&hotkey);
        }
        self.hook.unregister(hotkey)?;
        Ok(())
    }

    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut map_to_save = HashMap::<&Hotkey, &Sound>::new();

        for key_value in &(self.sound_bindings) {
            map_to_save.insert(key_value.0, key_value.1.as_ref());
        }

        let to_save = serde_json::to_string(&(map_to_save))?;

        std::fs::write("save.json", to_save)?;
        Ok(())
    }

    pub fn load_from_save<P: AsRef<Path>>(& mut self ,path: P) -> Result<(), Box<dyn std::error::Error>> {
        let saved = std::fs::read_to_string(path)?;

        let saved_map: HashMap<Hotkey, Sound> = serde_json::from_str(&saved)?;
        for key_value in saved_map {
            self.bind_new(key_value.1, key_value.0)?;
        }

        Ok(())
    }

    pub fn sounds(&self) -> &HashMap<Hotkey, Arc<Sound>> {
        &self.sound_bindings
    }
}
