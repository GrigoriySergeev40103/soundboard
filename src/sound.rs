use rodio::{source::Source, Decoder, OutputStreamHandle};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Sound {
    pub name: String,
    path_to_file: PathBuf,
}

impl Sound {
    pub fn new(name: String, path_to_file: PathBuf) -> Sound {
        Sound { name, path_to_file }
    }

    pub fn play(&self, stream_handle: &OutputStreamHandle) {
        let file = File::open(&(self.path_to_file));

        match file {
            Ok(opened_file) => {
                let buf_reader = BufReader::new(opened_file);
                let decoder_res = Decoder::new(buf_reader);

                match decoder_res {
                    Ok(decoder) => {
                        let play_result = stream_handle.play_raw(decoder.convert_samples());

                        if let Err(error) = play_result {
                            eprintln!("An error occured while trying to play a sound({})", error);
                        }
                    }
                    Err(error) => {
                        eprintln!(
                            "An error occured while trying to create a decoder for audio ({0}), 
                                error message: {1}",
                            self.name, error
                        );
                    }
                }
            }
            Err(error) => {
                match self.path_to_file.to_str() {
                        Some(utf8_path) => eprintln!("Something went wrong went trying to open file at path: ({0})", utf8_path),
                        None => eprintln!("Something went wrong went trying to open audio file of a sound named: ({0})", self.name)
                    }

                eprintln!("Error message: {}", error);
            }
        }
    }
}
