use std::fs;
use exif;
use std::io::BufReader;
use exif::In;
use std::fs::File;
use std::thread;
use image::codecs::jpeg::JpegEncoder;
use std::io::BufWriter;
use rayon::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum FileCategory {
    Jpg,
    Raw,
    Video,
    Unknown,
}

impl FileCategory {
    fn from_extension(ext: &str) -> Self {
        match ext {
            ".jpg" | ".jpeg" | ".png" => FileCategory::Jpg,
            ".cr3" | ".cr2" => FileCategory::Raw,
            ".mp4" | ".mpeg" | ".m4v" | ".webm" | ".webp" | ".mkv" | ".avi" | ".wmv" => FileCategory::Video,
            _ => FileCategory::Unknown,
        }
    }

    fn dir_name(&self) -> Option<&'static str> {
        match self {
            FileCategory::Jpg => Some("jpg"),
            FileCategory::Raw => Some("raw"),
            FileCategory::Video => Some("videos"),
            FileCategory::Unknown => None,
        }
    }

    fn all_categories() -> &'static [FileCategory] {
        &[FileCategory::Jpg, FileCategory::Raw, FileCategory::Video]
    }
}

use std::env;
use std::path::Path;

fn get_file_extension(filename: &str) -> String {
    if let Some(index) = filename.rfind(".") {
        let path_len = filename.len();
        return filename[index..path_len].to_string().to_lowercase();
    }
    "".to_string()
}

fn check_for_category(category: FileCategory, base_path: &Path) -> bool {
    if let Ok(files) = fs::read_dir(base_path) {
        for file in files {
            if let Ok(file) = file {
                let file_name = file.file_name();
                let ext = if let Some(s) = file_name.to_str() {
                    get_file_extension(s)
                } else {
                    continue;
                };
                if FileCategory::from_extension(&ext) == category {
                    return true;
                }
            }
        }
    }
    false
}

fn create_dirs(base_path: &Path) {
    // Always try to create the thumbnails directory
    let thumb_dir = base_path.join("thumbnails");
    match fs::create_dir(&thumb_dir) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                println!("There was an error creating dir {:?}: {}", thumb_dir, e);
            }
        }
    }

    for category in FileCategory::all_categories() {
        if check_for_category(*category, base_path) {
            if let Some(dir) = category.dir_name() {
                let target_dir = base_path.join(dir);
                match fs::create_dir(&target_dir) {
                    Ok(_) => (),
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::AlreadyExists {
                            println!("There was an error creating dir {:?}: {}", target_dir, e);
                        }
                    }
                }
            }
        }
    }
}

fn move_file(file: &str, base_path: &Path) {
    let ext = get_file_extension(file);
    let category = FileCategory::from_extension(&ext);

    if let Some(dir) = category.dir_name() {
        let source_path = base_path.join(file);
        let target_path = base_path.join(dir).join(file);
        if let Err(e) = fs::rename(&source_path, &target_path) {
            println!("Could not move file {:?}: {}", source_path, e);
        }
    }
}

fn get_orientation(file_path: &Path) -> u32 {
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return 1,
    };

    let exif_reader = exif::Reader::new();
    let mut bufreader = BufReader::new(file);

    let exif = match exif_reader.read_from_container(&mut bufreader) {
        Ok(exif) => exif,
        Err(_) => return 1,
    };

    if let Some(field) = exif.get_field(exif::Tag::Orientation, In::PRIMARY) {
        field.value.get_uint(0).unwrap_or(1) // Default to 1 if not present
    } else {
        1
    }
}

fn generate_thumbnail(filename: &str, quality: usize, base_path: &Path) {
    let source_path = base_path.join(filename);
    let img_reader = match image::ImageReader::open(&source_path) {
        Ok(reader) => reader,
        Err(e) => {
            println!("Failed to open image {:?}: {}", source_path, e);
            return;
        }
    };

    let mut img = match img_reader.decode() {
        Ok(decoded) => decoded,
        Err(e) => {
            println!("Failed to decode image {:?}: {}", source_path, e);
            return;
        }
    };

    let orientation = get_orientation(&source_path);
    if orientation == 8 {
        // Rotate if it is a vertical image
        img = img.rotate270();
    }

    img = img.resize(1920, 1080, image::imageops::FilterType::Lanczos3);
    
    let thumb_path = base_path.join("thumbnails").join(format!("thumb_{}", filename));
    let file = match File::create(&thumb_path) {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to create thumbnail for {:?}: {}", thumb_path, e);
            return;
        }
    };
    let mut writer = BufWriter::new(file);

    let mut encoder = JpegEncoder::new_with_quality(&mut writer, quality as u8);
    if let Err(e) = encoder.encode_image(&img) {
        println!("Failed to encode thumbnail for {:?}: {}", thumb_path, e);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let base_path = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        Path::new(".")
    };

    if !base_path.exists() || !base_path.is_dir() {
        println!("The path {:?} is not a valid directory.", base_path);
        return;
    }

    create_dirs(base_path);

    let num_threads = thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();

    let mut jpg_files = vec![];
    let mut other_files = vec![];

    if let Ok(files) = fs::read_dir(base_path) {
        for file in files {
            if let Ok(file) = file {
                if let Ok(file_type) = file.file_type() {
                    if file_type.is_file() {
                        let file_name_os = file.file_name();
                        let file_name = match file_name_os.to_str() {
                            Some(s) => s,
                            None => {
                                println!("Invalid UTF-8 in filename: {:?}", file_name_os);
                                continue;
                            }
                        };
                        let ext = get_file_extension(file_name);
                        let category = FileCategory::from_extension(&ext);

                        match category {
                            FileCategory::Jpg => {
                                jpg_files.push(file_name.to_string());
                            }
                            FileCategory::Raw | FileCategory::Video => {
                                other_files.push(file_name.to_string());
                            }
                            FileCategory::Unknown => {}
                        }
                    }
                } else {
                    println!("There was an error on file: {:?}", file.path())
                }
            }
        }
    }

    pool.install(|| {
        jpg_files.into_par_iter().for_each(|name| {
            generate_thumbnail(&name, 60, base_path);
            move_file(&name, base_path);
        });
    });

    for name in other_files {
        move_file(&name, base_path);
    }
}
