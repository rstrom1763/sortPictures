use std::{fs};
use exif;
use std::io::BufReader;
use exif::{In};
use std::fs::File;
use std::thread;
use image::codecs::jpeg::JpegEncoder;
use std::io::BufWriter;

fn get_file_extension(filename: &str) -> String {

    if let Some(index) = filename.rfind(".") {

        let path_len = filename.len();

       return filename[index..path_len].to_string().to_lowercase();

    }

    "".to_string()
}

fn check_for_type(types: Vec<&str>) -> bool {

    if let Ok(files) = fs::read_dir("./") {

        for file in files {

            if let Ok(file) = file {

                let file_name = file.file_name();

                let ext = &*get_file_extension(file_name.to_str().unwrap());
                if types.contains(&ext){ return true}

            }
        }
    }
    false
}

fn create_dirs() {

    let jpg_types = vec![".jpg", ".jpeg", ".png"];
    let raw_types = vec![".cr3", ".cr2"];
    let video_types = vec![".mp4", ".mpeg",".m4v",".webm",".webp",".mkv",".avi",".wmv"];

    if check_for_type(jpg_types) {
        match fs::create_dir("jpg") {
            Ok(_) => (),
            Err(e) => println!("There was an error creating dir: {}",e),
        }
    }

    if check_for_type(raw_types) {
        match fs::create_dir("raw") {
            Ok(_) => (),
            Err(e) => println!("There was an error creating dir: {}",e),
        }
    }

    if check_for_type(video_types) {
        match fs::create_dir("videos") {
            Ok(_) => (),
            Err(e) => println!("There was an error creating dir: {}",e),
        }
    }

}

fn move_file(file: &str) {

    let jpg_types = vec![".jpg", ".jpeg", ".png"];
    let raw_types = vec![".cr3", ".cr2"];
    let video_types = vec![".mp4", ".mpeg",".m4v",".webm",".webp",".mkv",".avi",".wmv"];

    let ext = &*get_file_extension(file);

    if jpg_types.contains(&ext){

        let target_path = ["./jpg/", file].concat();
        fs::rename(file, target_path)
            .expect(&*["Could not move file: ", file].concat());

    } else if raw_types.contains(&ext){

        let target_path = ["./raw/", file].concat();
        fs::rename(file, target_path)
            .expect(&*["Could not move file: ", file].concat());

    } else if video_types.contains(&ext){

        let target_path = ["./videos/", file].concat();
        fs::rename(file, target_path)
            .expect(&*["Could not move file: ", file].concat());

    }
}

fn get_orientation(file_path: &str) -> u32 {

    let mut orientation: u32 = 0;
    let file = File::open(file_path).unwrap();

    let exif_reader = exif::Reader::new();
    let mut bufreader = BufReader::new(file);

    let exif = exif_reader
        .read_from_container(&mut bufreader)
        .expect("Could not read from buffer");

    if let Some(field) = exif.get_field(exif::Tag::Orientation, In::PRIMARY) {
        orientation = field.value.get_uint(0).unwrap_or(1); // Default to 1 if not present
    }

    orientation
}

fn generate_thumbnail(filename: &str,quality: usize) {

    let mut img = image::ImageReader::open(filename)
        .expect("Failed to load image")
        .decode()
        .expect("Failed to decode image");


    let orientation = get_orientation(filename);
    if orientation == 8 { // Rotate if it is a vertical image
        img = img.rotate270();
    }

    img = img.resize(1920, 1080, image::imageops::FilterType::Lanczos3);

    let file = File::create(["thumb_",filename].concat()).unwrap();
    let mut writer = BufWriter::new(file);

    let mut encoder = JpegEncoder::new_with_quality(&mut writer, quality as u8);
    encoder.encode_image(&img).unwrap()
}

fn main() {

    create_dirs();

    let mut handles  = vec![];

    if let Ok(files) = fs::read_dir("./") {

        for file in files {

            if let Ok(file) = file {

                if let Ok(file_type) = file.file_type() {

                    if file_type.is_file() {

                        let ext = get_file_extension(file.file_name().to_str().unwrap());

                        if ext == ".jpg" {
                            handles.push(thread::spawn(move || {
                                generate_thumbnail(file.file_name().to_str().unwrap(),60);

                                move_file(&file
                                    .file_name()
                                    .to_str()
                                    .unwrap());
                            }));
                        } else {
                            move_file(&file
                                .file_name()
                                .to_str()
                                .unwrap());
                        }
                    }

                } else {

                    println!("There was an error on file: {:?}",file.path())

                }
            }
        }
    }

    for handle in handles{
        handle.join().unwrap();
    }

}
