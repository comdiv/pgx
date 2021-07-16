use crate::commands::get::find_control_file;
use crate::commands::get::get_property;
use pgx_utils::pg_config::PgConfig;
use pgx_utils::{exit_with_error, handle_result};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

pub(crate) fn generate_schema(
    pg_config: &PgConfig,
    is_release: bool,
    additional_features: &[&str],
    path: impl AsRef<std::path::Path>,
    dot: Option<impl AsRef<std::path::Path>>,
    verbose: bool,
) -> Result<(), std::io::Error> {
    let (control_file, _extname) = find_control_file();
    let major_version = pg_config.major_version()?;

    // Ensure the SQL generator exists.
    let cargo_toml = {
        let mut buf = String::default();
        let mut cargo_file =
            std::fs::File::open("Cargo.toml").expect(&format!("Could not open Cargo.toml"));
        cargo_file
            .read_to_string(&mut buf)
            .expect(&format!("Could not read Cargo.toml"));
        buf
    };
    let crate_name = cargo_toml
        .lines()
        .find(|line| line.starts_with("name"))
        .and_then(|line| line.split(" = ").last())
        .map(|line| line.trim_matches('\"').to_string())
        .expect("Expected crate name");
    let expected_bin_source_content = format!(
        "\
        /* Auto-generated by pgx. You may edit this, or delete it to have a new one created. */\n\
        pgx::pg_binary_magic!({});\n\
    ",
        crate_name
    );
    std::fs::create_dir_all("src/bin").expect("Could not create bin dir.");
    let generator_source_path = "src/bin/sql-generator.rs";
    let mut sql_gen_source_file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&generator_source_path)
        .expect(&format!("Could not open `{}`.", generator_source_path));
    let mut current_bin_source_content = String::default();
    sql_gen_source_file
        .read_to_string(&mut current_bin_source_content)
        .expect(&format!("Couldn't read {}.", generator_source_path));
    if current_bin_source_content != expected_bin_source_content {
        println!("
            `{}` does not exist or is not what is expected.\n\
            If you encounter problems please delete it and use the generated version.\
        ", generator_source_path)
    }
    if current_bin_source_content == "" {
        println!(
            "Created `{}` with content ```\n{}\n```.",
            generator_source_path, expected_bin_source_content
        );
        sql_gen_source_file
            .write_all(expected_bin_source_content.as_bytes())
            .expect("Couldn't write bin source.");
    }
    drop(sql_gen_source_file);

    if get_property("relocatable") != Some("false".into()) {
        exit_with_error!(
            "{}:  The `relocatable` property MUST be `false`.  Please update your .control file.",
            control_file.display()
        )
    }

    let mut features =
        std::env::var("PGX_BUILD_FEATURES").unwrap_or(format!("pg{}", major_version));
    let flags = std::env::var("PGX_BUILD_FLAGS").unwrap_or_default();
    if !additional_features.is_empty() {
        use std::fmt::Write;
        let mut additional_features = additional_features.join(" ");
        let _ = write!(&mut additional_features, " {}", features);
        features = additional_features
    }
    let mut command = Command::new("cargo");
    command.args(&["run", "--bin", "sql-generator"]);
    if is_release {
        command.arg("--release");
    }

    if verbose {
        command.env("RUST_LOG", "debug");
    }

    if !features.trim().is_empty() {
        command.arg("--features");
        command.arg(&features);
        command.arg("--no-default-features");
    }

    for arg in flags.split_ascii_whitespace() {
        command.arg(arg);
    }

    let path = path.as_ref();
    let _ = path.parent().map(|p| std::fs::create_dir_all(&p).unwrap());
    command.arg("--");
    command.arg(path);
    if let Some(dot) = dot {
        command.arg(dot.as_ref());
    }

    let command = command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let command_str = format!("{:?}", command);
    println!(
        "running SQL generator features `{}`\n{}",
        features, command_str
    );
    let status = handle_result!(
        command.status(),
        format!("failed to spawn cargo: {}", command_str)
    );
    if !status.success() {
        exit_with_error!("failed to run SQL generator");
    }
    Ok(())
}
