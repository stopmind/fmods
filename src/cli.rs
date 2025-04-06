use crate::cli::Commands::Info;
use crate::config::Config;
use crate::downloader::Downloader;
use crate::factorio_api::FactorioApi;
use crate::instance::Instance;
use crate::mod_info::Version;
use crate::utils::{process_dependencies, Changes};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::io::stdin;
use std::mem::take;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[arg(long)]
    ask: bool,

    #[arg(long)]
    no_ask: bool,

    #[arg(long)]
    instance: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Eq, PartialEq)]
enum Commands {
    /// Work with instances
    Instances {
        #[command(subcommand)]
        command: InstancesCommands
    },
    /// Info about instance
    Info,
    /// List installed mods
    List,
    /// Download mod
    Download {
        name: String,
        mod_version: Option<Version>,
    },
    /// Remove mod
    Remove {
        name: String,
    }
}

#[derive(Subcommand, Eq, PartialEq)]
enum InstancesCommands {
    /// Add new instance
    Add {
        name: String,
        path: PathBuf,

        #[arg(long)]
        replace: bool,

        #[arg(long)]
        default: bool
    },
    /// Remove an instance
    Remove {
        name: String
    },
    /// List all instances
    List,
    /// Set default instance
    Default {
        name: String
    },
    /// Unset default
    UnsetDefault
}

fn save_config(config: &Config) {
    config.save().unwrap();
}

fn choose(msg: String, variants: &[String]) -> String {
    loop {
        let mut input = String::new();
        println!("{}", msg);
        stdin().read_line(&mut input).unwrap();
        input = input.to_ascii_lowercase().trim_end().to_string();
        for variant in variants {
            if &input == variant {
                return input;
            }
        }
    }
}

pub fn cli() {
    let mut config = Config::load();

    let args = Args::parse();

    let ask = (config.ask || args.ask) && !args.no_ask;

    if let Commands::Instances {command} = &args.command {
        match command {
            InstancesCommands::Add {name, path, replace, default } => {
                if config.instances.contains_key(name) && !replace {
                    println!("The instance \"{}\" already exists.", name);
                    if !ask {
                        return;
                    }

                    let answer = choose(format!("Do you want replace instance? ({}es/{}o)", "y".bold(), "n".bold()),
                                        &["y".to_string(), "n".to_string()]);
                    match answer.as_str() {
                        "n" => return,
                        _ => {}
                    }
                }

                match Instance::new(path.clone()) {
                    Ok(instance) => instance_info(&instance, name),
                    Err(err) => return println!("Failed to open instance: {}", err)
                }

                config.instances.insert(name.clone(), path.clone());
                if *default {
                    config.default_instance = Some(name.clone());
                }

                save_config(&config);

            }
            InstancesCommands::Remove {name} => {
                if config.instances.contains_key(name) {
                    config.instances.remove(name);

                    if let Some(default_instance) = &config.default_instance {
                        if default_instance == name {
                            config.default_instance = None;
                        }
                    }

                    println!("The instance \"{}\" is removed", name);
                    save_config(&config);
                } else {
                    println!("A instance with the name \"{}\" was not found.", name);
                }
            }
            InstancesCommands::List => {
                println!("Default instance: {}", match &config.default_instance {
                    None => "not specified".bright_black(),
                    Some(str) => str.bright_yellow()
                });
                println!("Saved {} instances:", config.instances.len().to_string().bright_blue());
                for instance in config.instances {
                    println!("  {} -> {}", instance.0.bright_yellow(), instance.1.to_string_lossy().bright_yellow());
                }
            }
            InstancesCommands::Default {name} => {
                if config.instances.contains_key(name) {
                    config.default_instance = Some(name.clone());
                    println!("The instance \"{}\" is default now.", name);
                    save_config(&config);
                    return;
                }

                println!("A instance with the name \"{}\" was not found.", name);
            }
            InstancesCommands::UnsetDefault => {
                config.default_instance = None;
                println!("The default instance no specified now.");
                save_config(&config);
            }
        }

        return;
    }

    let instance_name = {
        let mut option = args.instance.clone();

        if option == None {
            option = config.default_instance.clone();
        }

        if option == None && ask {
            println!("Select instance");
            for instance in &config.instances {
                println!("  {}", instance.0.bright_yellow());
            }

            let mut name = String::new();
            stdin().read_line(&mut name).unwrap();
            option = Some(name.trim_end().to_string());
        }

        if let Some(name) = option {
            name
        } else {
            println!("No instance selected.");
            return;
        }
    };

    let instance = match config.instances.get(&instance_name) {
        None => return println!("A instance with the name \"{}\" was not found.", instance_name),
        Some(value) => match Instance::new(value.clone()) {
            Ok(instance) => instance,
            Err(err) => return println!("Failed to open instance \"{}\": {}", instance_name, err)
        },
    };

    match args.command {
        Info => instance_info(&instance, &instance_name),
        Commands::List => {
            println!("Installed {} mods:", instance.mods.len().to_string().bright_blue());
            for mod_info in &instance.mods {
                println!("  {} {}", mod_info.name.bright_yellow(), mod_info.version.to_string().bright_yellow());
            }
        }
        Commands::Download { name, mod_version: version } => {
            let factorio_api = FactorioApi::new(&instance);

            let mod_info = match factorio_api.get_mod(&name) {
                Ok(val) => val,
                Err(err) => return println!("Failed to fetch mod: {}", err)
            };

            if mod_info.releases.len() == 0 {
                println!("No suitable releases found");
                return;
            }

            let version = match version {
                Some(val) => val,
                None => {
                    println!("Select version:");
                    for release in mod_info.releases {
                        println!("  {}", release.version.to_string().bright_yellow());
                    }

                    let mut name = String::new();
                    stdin().read_line(&mut name).unwrap();

                    match Version::from_str(name.trim_end()) {
                        Ok(ver) => ver,
                        Err(err) => return println!("Failed to parse version: {}", err)
                    }
                }
            };

            println!("Processing dependencies...");

            let dependencies = match process_dependencies(&factorio_api, &instance, name, version) {
                Ok(vec) => vec,
                Err(err) => return println!("Failed to process dependencies: {}", err),
            };

            let mut changes = Changes::compute(&instance, &dependencies);

            println!("Install ({}):", changes.install.len().to_string().bright_green());
            for install in &changes.install {
                println!("  {} {}", install.id.to_string().bright_yellow(), install.version.to_string().bright_yellow());
            }

            println!("Update ({}):", changes.update.len().to_string().bright_yellow());
            for update in &changes.update {
                println!("  {} {} -> {}", update.id.to_string().bright_yellow(), update.old_version.to_string().bright_yellow(),
                    update.new_version.to_string().bright_yellow()
                );
            }

            println!("Conflicts ({}):", changes.conflicts.len().to_string().bright_red());
            for conflict in &changes.conflicts {
                println!("  {}", conflict.bright_yellow());
            }

            let downloader = Downloader::new(&instance);

            let answer = choose(format!("Proceed? ({}es/{}o)", "y".bold(), "n".bold()),
                                &["y".to_string(), "n".to_string()]);
            match answer.as_str() {
                "n" => return,
                _ => {}
            }

            println!("Downloading...");
            for install in take(&mut changes.install) {
                if let Err(err) = downloader.download(install.id, install.version) {
                    println!("Failed to download: {}", err);
                    return;
                }
            }

            println!("Updating...");
            for update in take(&mut changes.update) {
                instance.remove_mod(update.id.as_str());

                if let Err(err) = downloader.download(update.id, update.new_version) {
                    println!("Failed to download: {}", err);
                    return;
                }
            }

            println!("Removing conflicts...");
            for conflict in take(&mut changes.conflicts) {
                instance.remove_mod(conflict.as_str());
            }

            println!("{}", "\nDone!".bright_green().bold());
        }
        Commands::Remove { name } => {
            if let Some(_) = instance.mods.iter().find(|x| x.name == name) {
                instance.remove_mod(name.as_str());
                println!("The mod \"{}\" was removed", name);
            } else {
                println!("The mod \"{}\" was not found.", name);
            }
        }
        _ => {}
    }

}

fn instance_info(instance: &Instance, instance_name: &String) {
    println!("\
Instance:       {}\n\
Path:           {}\n\
Version:        {}\n\
Mods installed: {}\n\
Game content versions:",
             instance_name.bright_yellow(), instance.path.to_string_lossy().bright_yellow(),
             instance.version.to_string().bright_yellow(), instance.mods.len().to_string().bright_yellow());

    for game_content_version in &instance.game_content_versions {
        println!("  {} {}", game_content_version.0, game_content_version.1.to_string().bright_yellow())
    }
}