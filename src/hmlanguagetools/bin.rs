use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
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
        #[clap(default_value_t = false)]
        hex_precision: bool,

        #[clap(long)]
        default_locale: Option<String>,

        #[clap(long)]
        #[clap(default_value_t = false)]
        symmetric: bool,
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

        #[clap(default_value_t = false)]
        recursive: bool,
    },
    Rebuild {
        input_folder: PathBuf,

        output_folder: PathBuf,

        #[clap(default_value_t = false)]
        recursive: bool,
    },
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

    let hashlist_data = fs::read("hash_list.hmla");
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

            let meta_json =
                fs::read_to_string(meta_path.unwrap()).expect("Failed to read meta file.");

            match args.file_type {
                Filetype::CLNG => {
                    let clng = hmlanguages::clng::CLNG::new(version, lang_map)
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
                        println!("Failed to parse CLNG file.");
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
                        println!("Failed to parse DITL file.");
                        return 1;
                    }
                }
                Filetype::DLGE => {
                    let dlge = hmlanguages::dlge::DLGE::new(hashlist, version, lang_map, default_locale, hex_precision)
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
                        println!("Failed to parse DLGE file.");
                        return 1;
                    }
                }
                Filetype::LOCR => {
                    let locr = hmlanguages::locr::LOCR::new(hashlist, version, lang_map, symmetric)
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
                        println!("Failed to parse LOCR file.");
                        return 1;
                    }
                }
                Filetype::RTLV => {
                    let rtlv = hmlanguages::rtlv::RTLV::new(version, lang_map)
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
                        println!("Failed to parse RTLV file.");
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

            match args.file_type {
                Filetype::CLNG => {
                    let clng = hmlanguages::clng::CLNG::new(version, lang_map)
                        .expect("Failed to get rebuilder for CLNG.");

                    let json = clng.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(clng) = json {
                        fs::write(output, clng.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, clng.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild CLNG file.");
                        return 1;
                    }
                }
                Filetype::DITL => {
                    let mut ditl = hmlanguages::ditl::DITL::new(hashlist)
                        .expect("Failed to get rebuilder for DITL.");

                    let json = ditl.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(ditl) = json {
                        fs::write(output, ditl.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, ditl.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild DITL file.");
                        return 1;
                    }
                }
                Filetype::DLGE => {
                    let mut dlge =
                        hmlanguages::dlge::DLGE::new(hashlist, version, lang_map, default_locale, false)
                            .expect("Failed to get rebuilder for DLGE.");

                    let json = dlge.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(dlge) = json {
                        fs::write(output, dlge.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, dlge.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild DLGE file.");
                        return 1;
                    }
                }
                Filetype::LOCR => {
                    let locr = hmlanguages::locr::LOCR::new(hashlist, version, lang_map, symmetric)
                        .expect("Failed to get rebuilder for LOCR.");

                    let json = locr.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(locr) = json {
                        fs::write(output, locr.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, locr.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild LOCR file.");
                        return 1;
                    }
                }
                Filetype::RTLV => {
                    let mut rtlv = hmlanguages::rtlv::RTLV::new(version, lang_map)
                        .expect("Failed to get rebuilder for RTLV.");

                    let json = rtlv.rebuild(
                        String::from_utf8(
                            std::fs::read(input).expect("Failed to read input file."),
                        )
                        .expect("Failed to utf-8 convert input file."),
                    );

                    if let Ok(rtlv) = json {
                        fs::write(output, rtlv.file.as_slice())
                            .expect("Failed to write rebuilt file.");
                        fs::write(out_meta_path, rtlv.meta)
                            .expect("Failed to write rebuilt meta file.");
                    } else {
                        println!("Failed to rebuild RTLV file.");
                        return 1;
                    }
                }
            }

            println!("Rebuilt JSON to {:?}!", args.file_type);
        }
        Commands::Batch { batch } => match batch {
            BatchCommands::Convert {
                input_folder,
                output_folder,
                recursive,
            } => {
                if !input_folder.exists() {
                    println!("Input folder is invalid.");
                    return 1;
                }
            }
            BatchCommands::Rebuild {
                input_folder,
                output_folder,
                recursive,
            } => {
                if !input_folder.exists() {
                    println!("Input folder is invalid.");
                    return 1;
                }
            }
        },
    }

    return 0;
}
