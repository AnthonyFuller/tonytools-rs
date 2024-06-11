use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use glob::glob;
use tonytools::{hashlist::HashList, hmlanguages, Version};

#[derive(ValueEnum, Clone, Debug)]
enum GameVersion {
    H3,
    H2,
    H2016,
}

#[derive(ValueEnum, Clone, Debug)]
enum Filetype {
    CLNG,
    DLGE,
    DITL,
    LOCR,
    RTLV,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(value_enum)]
    version: GameVersion,

    #[arg(value_enum)]
    file_type: Filetype,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Convert {
        input: PathBuf,

        output: PathBuf,

        #[clap(long)]
        meta_path: Option<PathBuf>,

        #[clap(long)]
        lang_map: Option<String>,

        #[clap(long)]
        default_locale: Option<String>,

        #[clap(long)]
        #[clap(default_value_t = false)]
        symmetric: bool,

        #[clap(long)]
        #[clap(default_value_t = false)]
        hex_precision: bool,
    },
    Rebuild {
        input: PathBuf,

        output: PathBuf,

        #[clap(long)]
        meta_path: Option<PathBuf>,

        #[clap(long)]
        lang_map: Option<String>,

        #[clap(long)]
        default_locale: Option<String>,

        #[clap(long)]
        #[clap(default_value_t = false)]
        symmetric: bool,
    },
    Batch {
        #[command(subcommand)]
        batch: BatchCommands,
    },
}

#[derive(Subcommand, Debug)]
enum BatchCommands {
    Convert {
        input_folder: PathBuf,

        output_folder: PathBuf,

        #[clap(long)]
        #[clap(default_value_t = false)]
        recursive: bool,

        #[clap(long)]
        lang_map: Option<String>,

        #[clap(long)]
        default_locale: Option<String>,

        #[clap(long)]
        #[clap(default_value_t = false)]
        symmetric: bool,

        #[clap(long)]
        #[clap(default_value_t = false)]
        hex_precision: bool,
    },
    Rebuild {
        input_folder: PathBuf,

        output_folder: PathBuf,

        #[clap(long)]
        #[clap(default_value_t = false)]
        recursive: bool,

        #[clap(long)]
        lang_map: Option<String>,

        #[clap(long)]
        default_locale: Option<String>,

        #[clap(long)]
        #[clap(default_value_t = false)]
        symmetric: bool,
    },
}

enum Converter {
    CLNG(hmlanguages::clng::CLNG),
    DITL(hmlanguages::ditl::DITL),
    DLGE(hmlanguages::dlge::DLGE),
    RTLV(hmlanguages::rtlv::RTLV),
    LOCR(hmlanguages::locr::LOCR),
}

impl Converter {
    fn new(
        file_type: Filetype,
        hashlist: HashList,
        version: Version,
        lang_map: Option<Vec<String>>,
        default_locale: Option<String>,
        hex_precision: bool,
        symmetric: bool,
    ) -> Self {
        match file_type {
            Filetype::CLNG => {
                let converter = hmlanguages::clng::CLNG::new(version, lang_map)
                    .expect("Failed to get converter for CLNG.");
                Converter::CLNG(converter)
            }
            Filetype::DITL => {
                let converter = hmlanguages::ditl::DITL::new(hashlist)
                    .expect("Failed to get converter for DITL.");
                Converter::DITL(converter)
            }
            Filetype::DLGE => {
                let converter = hmlanguages::dlge::DLGE::new(hashlist, version, lang_map, default_locale, hex_precision)
                    .expect("Failed to get converter for DLGE.");
                Converter::DLGE(converter)
            }
            Filetype::RTLV => {
                let converter = hmlanguages::rtlv::RTLV::new(version, lang_map)
                    .expect("Failed to get converter for RTLV.");
                Converter::RTLV(converter)
            }
            Filetype::LOCR => {
                let converter = hmlanguages::locr::LOCR::new(hashlist, version, lang_map, symmetric)
                    .expect("Failed to get converter for LOCR.");
                Converter::LOCR(converter)
            }
        }
    }
}

fn main() {
    let exit_code = real_main();
    std::process::exit(exit_code);
}

fn real_main() -> i32 {
    let args = Args::parse();

    let version = match args.version {
        GameVersion::H3 => Version::H3,
        GameVersion::H2 => Version::H2,
        GameVersion::H2016 => Version::H2016,
    };

    let mut hashlist_path = std::env::current_exe().expect("Failed to get current exe path.");
    hashlist_path.pop();
    hashlist_path.push("hash_list.hmla");

    let hashlist_data = fs::read(hashlist_path);
    if hashlist_data.is_err() {
        println!("Hash list not found!");
        return 1;
    }
    let hashlist = HashList::load(&hashlist_data.unwrap()).expect("Failed to load hash list.");

    match args.cmd {
        Commands::Convert {
            input,
            output,
            mut meta_path,
            lang_map,
            hex_precision,
            default_locale,
            symmetric,
        } => {
            if !input.exists() {
                println!("Input path is invalid.");
                return 1;
            }

            if !meta_path.as_ref().is_some_and(|path| path.exists()) {
                println!("Meta path does not exist. Trying input + .meta.JSON");
                meta_path = Some(PathBuf::from(format!(
                    "{}.meta.JSON",
                    input.to_str().unwrap()
                )));

                if !meta_path.as_ref().unwrap().exists() {
                    println!("Could not find meta!");
                    return 1;
                }
            }

            let lang_map_vec: Option<Vec<String>> = lang_map.map(|map| map.split(',').map(|s| s.to_string()).collect());

            let meta_json =
                fs::read_to_string(meta_path.unwrap()).expect("Failed to read meta file.");

            match args.file_type {
                Filetype::CLNG => {
                    let clng = hmlanguages::clng::CLNG::new(version, lang_map_vec)
                        .expect("Failed to get converter for CLNG.");

                    let json = clng.convert(
                        fs::read(input)
                            .expect("Failed to read input file.")
                            .as_slice(),
                        meta_json,
                    );

                    if let Ok(clng) = json {
                        fs::write(output, serde_json::to_string(&clng).unwrap())
                            .expect("Failed to write converted JSON.");
                    } else {
                        println!("Failed to parse CLNG file {:?}.", json.unwrap_err());
                        return 1;
                    }
                }
                Filetype::DITL => {
                    let ditl = hmlanguages::ditl::DITL::new(hashlist)
                        .expect("Failed to get converter for DITL.");

                    let json = ditl.convert(
                        fs::read(input)
                            .expect("Failed to read input file.")
                            .as_slice(),
                        meta_json,
                    );

                    if let Ok(ditl) = json {
                        fs::write(output, serde_json::to_string(&ditl).unwrap())
                            .expect("Failed to write converted JSON.");
                    } else {
                        println!("Failed to parse DITL file {:?}.", json.unwrap_err());
                        return 1;
                    }
                }
                Filetype::DLGE => {
                    let dlge = hmlanguages::dlge::DLGE::new(
                        hashlist,
                        version,
                        lang_map_vec,
                        default_locale,
                        hex_precision,
                    )
                    .expect("Failed to get converter for DLGE.");

                    let json = dlge.convert(
                        fs::read(input)
                            .expect("Failed to read input file.")
                            .as_slice(),
                        meta_json,
                    );

                    if let Ok(dlge) = json {
                        fs::write(output, serde_json::to_string(&dlge).unwrap())
                            .expect("Failed to write converted JSON.");
                    } else {
                        println!("Failed to parse DLGE file: {:?}.", json.unwrap_err());
                        return 1;
                    }
                }
                Filetype::LOCR => {
                    let locr = hmlanguages::locr::LOCR::new(hashlist, version, lang_map_vec, symmetric)
                        .expect("Failed to get converter for LOCR.");

                    let json = locr.convert(
                        fs::read(input)
                            .expect("Failed to read input file.")
                            .as_slice(),
                        meta_json,
                    );

                    if let Ok(locr) = json {
                        fs::write(output, serde_json::to_string(&locr).unwrap())
                            .expect("Failed to write converted JSON.");
                    } else {
                        println!("Failed to parse LOCR file {:?}.", json.unwrap_err());
                        return 1;
                    }
                }
                Filetype::RTLV => {
                    let rtlv = hmlanguages::rtlv::RTLV::new(version, lang_map_vec)
                        .expect("Failed to get converter for RTLV.");

                    let json = rtlv.convert(
                        fs::read(input)
                            .expect("Failed to read input file.")
                            .as_slice(),
                        meta_json,
                    );

                    if let Ok(rtlv) = json {
                        fs::write(output, serde_json::to_string(&rtlv).unwrap())
                            .expect("Failed to write converted JSON.");
                    } else {
                        println!("Failed to parse RTLV file {:?}.", json.unwrap_err());
                        return 1;
                    }
                }
            }

            println!("Converted {:?} to JSON!", args.file_type);
        }
        Commands::Rebuild {
            input,
            output,
            meta_path,
            lang_map,
            default_locale,
            symmetric,
        } => {
            if !input.exists() {
                println!("Input path is invalid.");
                return 1;
            }

            let out_meta_path = if meta_path.is_some() {
                meta_path.unwrap()
            } else {
                PathBuf::from(format!("{}.meta.JSON", input.to_str().unwrap()))
            };

            let lang_map_vec: Option<Vec<String>> = lang_map.map(|map| map.split(',').map(|s| s.to_string()).collect());

            match args.file_type {
                Filetype::CLNG => {
                    let clng = hmlanguages::clng::CLNG::new(version, lang_map_vec)
                        .expect("Failed to get rebuilder for CLNG.");

                    let rebuilt = clng.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(clng) = rebuilt {
                        fs::write(output, clng.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, clng.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild CLNG file {:?}.", rebuilt.unwrap_err());
                        return 1;
                    }
                }
                Filetype::DITL => {
                    let mut ditl = hmlanguages::ditl::DITL::new(hashlist)
                        .expect("Failed to get rebuilder for DITL.");

                    let rebuilt = ditl.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(ditl) = rebuilt {
                        fs::write(output, ditl.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, ditl.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild DITL file {:?}.", rebuilt.unwrap_err());
                        return 1;
                    }
                }
                Filetype::DLGE => {
                    let mut dlge = hmlanguages::dlge::DLGE::new(
                        hashlist,
                        version,
                        lang_map_vec,
                        default_locale,
                        false,
                    )
                    .expect("Failed to get rebuilder for DLGE.");

                    let rebuilt = dlge.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(dlge) = rebuilt {
                        fs::write(output, dlge.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, dlge.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild DLGE file {:?}.", rebuilt.unwrap_err());
                        return 1;
                    }
                }
                Filetype::LOCR => {
                    let locr = hmlanguages::locr::LOCR::new(hashlist, version, lang_map_vec, symmetric)
                        .expect("Failed to get rebuilder for LOCR.");

                    let rebuilt = locr.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(locr) = rebuilt {
                        fs::write(output, locr.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, locr.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild LOCR file {:?}.", rebuilt.unwrap_err());
                        return 1;
                    }
                }
                Filetype::RTLV => {
                    let mut rtlv = hmlanguages::rtlv::RTLV::new(version, lang_map_vec)
                        .expect("Failed to get rebuilder for RTLV.");

                    let rebuilt = rtlv.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(rtlv) = rebuilt {
                        fs::write(output, rtlv.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, rtlv.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild RTLV file {:?}.", rebuilt.unwrap_err());
                        return 1;
                    }
                }
            }

            println!("Rebuilt JSON to {:?}!", args.file_type);
        }
        Commands::Batch { batch } => match batch {
            BatchCommands::Convert {
                mut input_folder,
                output_folder,
                recursive,
                lang_map,
                default_locale,
                symmetric,
                hex_precision,
            } => {
                if !input_folder.exists() {
                    println!("Input folder is invalid.");
                    return 1;
                }

                if !output_folder.exists() && fs::create_dir_all(output_folder.clone()).is_err() {
                    println!("Failed to create output folder.");
                    return 1;
                }

                let lang_map_vec: Option<Vec<String>> = lang_map.map(|map| map.split(',').map(|s| s.to_string()).collect());

                if recursive {
                    input_folder.push("**")
                }

                let ext = match args.file_type {
                    Filetype::CLNG => "CLNG",
                    Filetype::DITL => "DITL",
                    Filetype::DLGE => "DLGE",
                    Filetype::LOCR => "LOCR",
                    Filetype::RTLV => "RTLV",
                };

                input_folder.push(format!("*.{}", ext));

                let converter = Converter::new(
                    args.file_type,
                    hashlist,
                    version,
                    lang_map_vec,
                    default_locale,
                    hex_precision,
                    symmetric
                );

                for entry in glob(input_folder.to_str().expect("Failed to convert path.")).expect("Failed to read glob pattern") {
                    if let Err(e) = entry {
                        println!("Invalid path - \"{:?}\"", e);
                        continue;
                    }

                    let path = entry.unwrap();

                    let data = fs::read(path.clone());
                    if let Err(e) = data {
                        println!("Failed to load file - \"{:?}\"", e);
                        continue;
                    }

                    let meta_json = fs::read_to_string(PathBuf::from(format!("{}.meta.JSON", path.to_str().unwrap())));
                    if let Err(e) = meta_json {
                        println!("Failed to load meta - \"{:?}\"", e);
                        continue;
                    }

                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    
                    let json = match converter {
                        Converter::CLNG(ref converter) => {
                            let clng = converter.convert(data.unwrap().as_slice(), meta_json.unwrap());
                            if let Err(e) = clng {
                                println!("Failed to convert file - \"{:?}\"", e);
                                continue;
                            }

                            serde_json::to_string(&clng.unwrap()).expect("Failed to convert JSON to string.")
                        }
                        Converter::DLGE(ref converter) => {
                            let dlge = converter.convert(data.unwrap().as_slice(), meta_json.unwrap());
                            if let Err(e) = dlge {
                                println!("Failed to convert file - \"{:?}\"", e);
                                continue;
                            }

                            serde_json::to_string(&dlge.unwrap()).expect("Failed to convert JSON to string.")
                        }
                        Converter::LOCR(ref converter) => {
                            let locr = converter.convert(data.unwrap().as_slice(), meta_json.unwrap());
                            if let Err(e) = locr {
                                println!("Failed to convert file - \"{:?}\"", e);
                                continue;
                            }

                            serde_json::to_string(&locr.unwrap()).expect("Failed to convert JSON to string.")
                        }
                        Converter::DITL(ref converter) => {
                            let ditl = converter.convert(data.unwrap().as_slice(), meta_json.unwrap());
                            if let Err(e) = ditl {
                                println!("Failed to convert file - \"{:?}\"", e);
                                continue;
                            }

                            serde_json::to_string(&ditl.unwrap()).expect("Failed to convert JSON to string.")
                        }
                        Converter::RTLV(ref converter) => {
                            let rtlv = converter.convert(data.unwrap().as_slice(), meta_json.unwrap());
                            if let Err(e) = rtlv {
                                println!("Failed to convert file - \"{:?}\"", e);
                                continue;
                            }

                            serde_json::to_string(&rtlv.unwrap()).expect("Failed to convert JSON to string.")
                        }
                    };

                    let mut output_path = output_folder.clone();
                    output_path.push(file_name);
                    output_path.set_extension(format!("{}.json", ext.to_lowercase()));

                    if let Err(e) = fs::write(output_path, json) {
                        println!("Failed to write output file - \"{:?}\"", e);
                        continue;
                    }

                    println!("Processed {:?}", file_name);
                }
            }
            BatchCommands::Rebuild {
                mut input_folder,
                output_folder,
                recursive,
                lang_map,
                default_locale,
                symmetric,
            } => {
                if !input_folder.exists() {
                    println!("Input folder is invalid.");
                    return 1;
                }

                if !output_folder.exists() && fs::create_dir_all(output_folder.clone()).is_err() {
                    println!("Failed to create output folder.");
                    return 1;
                }

                if recursive {
                    input_folder.push("**")
                }

                let lang_map_vec: Option<Vec<String>> = lang_map.map(|map| map.split(',').map(|s| s.to_string()).collect());

                let ext = match args.file_type {
                    Filetype::CLNG => "CLNG",
                    Filetype::DITL => "DITL",
                    Filetype::DLGE => "DLGE",
                    Filetype::LOCR => "LOCR",
                    Filetype::RTLV => "RTLV",
                };

                input_folder.push(format!("*.{}.json", ext.to_lowercase()));

                let mut converter = Converter::new(
                    args.file_type,
                    hashlist,
                    version,
                    lang_map_vec,
                    default_locale,
                    false,
                    symmetric
                );

                for entry in glob(input_folder.to_str().expect("Failed to convert path.")).expect("Failed to read glob pattern") {
                    if let Err(e) = entry {
                        println!("Invalid path - \"{:?}\"", e);
                        continue;
                    }

                    let path = entry.unwrap();

                    let file = fs::read(path.clone());
                    if let Err(e) = file {
                        println!("Failed to load file - \"{:?}\"", e);
                        continue;
                    }

                    let data = String::from_utf8(file.unwrap());
                    if let Err(e) = data {
                        println!("Failed to load JSON file - \"{:?}\"", e);
                        continue;
                    }

                    let file_name = path.file_name().unwrap().to_str().unwrap().split(".").collect::<Vec<&str>>()[0];
                    
                    let rebuilt = match converter {
                        Converter::CLNG(ref converter) => {
                            let clng = converter.rebuild(data.unwrap());
                            if let Err(e) = clng {
                                println!("Failed to rebuild file - \"{:?}\"", e);
                                continue;
                            }

                            clng.unwrap()
                        }
                        Converter::DLGE(ref mut converter) => {
                            let dlge = converter.rebuild(data.unwrap());
                            if let Err(e) = dlge {
                                println!("Failed to rebuild file - \"{:?}\"", e);
                                continue;
                            }

                            dlge.unwrap()
                        }
                        Converter::LOCR(ref converter) => {
                            let locr = converter.rebuild(data.unwrap());
                            if let Err(e) = locr {
                                println!("Failed to rebuild file - \"{:?}\"", e);
                                continue;
                            }

                            locr.unwrap()
                        }
                        Converter::DITL(ref mut converter) => {
                            let ditl = converter.rebuild(data.unwrap());
                            if let Err(e) = ditl {
                                println!("Failed to rebuild file - \"{:?}\"", e);
                                continue;
                            }

                            ditl.unwrap()
                        }
                        Converter::RTLV(ref mut converter) => {
                            let rtlv = converter.rebuild(data.unwrap());
                            if let Err(e) = rtlv {
                                println!("Failed to rebuild file - \"{:?}\"", e);
                                continue;
                            }

                            rtlv.unwrap()
                        }
                    };

                    let mut rebuilt_path = output_folder.clone();
                    rebuilt_path.push(file_name);
                    rebuilt_path.set_extension(ext);

                    let mut meta_path = output_folder.clone();
                    meta_path.push(file_name);
                    meta_path.set_extension(format!("{}.meta.JSON", ext));

                    if let Err(e) = fs::write(rebuilt_path, rebuilt.file) {
                        println!("Failed to write rebuilt file - \"{:?}\"", e);
                        continue;
                    }

                    if let Err(e) = fs::write(meta_path, rebuilt.meta) {
                        println!("Failed to write meta file - \"{:?}\"", e);
                        continue;
                    }

                    println!("Processed {:?}.{:?}.json", file_name, ext.to_lowercase());
                }
            }
        },
    }

    return 0;
}
