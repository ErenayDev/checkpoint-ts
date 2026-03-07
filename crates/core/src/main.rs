use clap::Parser;
use color_eyre::{Result, eyre};

pub mod services;
pub mod tui;
pub mod utils;

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use utils::discord_rpc::DiscordRpc;

use services::transformer::TransformService;
use tui::CheckpointTUI;
use utils::{entry_finder, project_context::ProjectContext};

use crate::utils::discord_rpc::{Activity, Button, RpcSession};

#[derive(Parser)]
#[command(name = "checkpoint")]
#[command(
    about = "Interactive checkpoint system for TypeScript/JavaScript development with time-travel capabilities"
)]
struct Cli {
    #[arg(short, long, help = "Input file or directory path")]
    input: std::path::PathBuf,

    #[arg(short, long, help = "Output path for transformed file")]
    output: Option<std::path::PathBuf>,

    #[arg(
        short,
        long,
        help = "Enable minification",
        default_value_t = false,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    minify: bool,

    #[arg(long, help = "Skip transformation, use cached files only")]
    no_transform: bool,

    #[arg(long, help = "Transform only entry file and its imports")]
    only_needed: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Cli::parse();

    let entry_file: PathBuf = match entry_finder::find_entry_file(args.input.to_str()) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("[ERROR]: {e}");
            std::process::exit(1);
        }
    };

    let ctx = ProjectContext::discover(&entry_file);
    ctx.ensure_checkpoint_dir()?;

    let minify = args.minify || ctx.config.minify.unwrap_or(false);

    if !args.no_transform {
        let mut transformer = TransformService::new(ctx.clone());

        if args.only_needed {
            println!("[INFO] Transforming entry file and dependencies...");
            transformer
                .transform_file(&entry_file, minify)
                .map_err(|e| eyre::eyre!("Transform failed: {}", e))?;
        } else {
            println!("[INFO] Transforming entire project...");
            let transformed = transformer
                .transform_project(minify)
                .map_err(|e| eyre::eyre!("Transform failed: {}", e))?;
            println!("[INFO] Transformed {} files", transformed.len());
        }
    }

    let transformed_entry: PathBuf = ctx
        .checkpoint_dir
        .join("transforms")
        .join(entry_file.file_name().unwrap());

    if args.no_transform && !transformed_entry.exists() {
        return Err(eyre::eyre!(
            "--no-transform requires an existing cached transform at {}",
            transformed_entry.display()
        ));
    }

    copy_runtime_files(&ctx)?;
    // discord rpc logic
    //
    // delete if u want

    let rpc = match DiscordRpc::connect("1232253585962700911") {
        Ok(rpc) => {
            println!("discord rpc connected");
            Some(rpc)
        }
        Err(_) => {
            println!("discord not running, skipping rpc");
            None
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut activity = Activity::new();
    activity.large_image = Some("logo".into());
    activity.large_text = Some("Checkpoint.ts".into());
    activity.start_timestamp = Some(now);
    activity.buttons = Some(vec![Button {
        label: "GitHub".into(),
        url: "https://github.com/ErenayDev/Checkpoint-ts".into(),
    }]);

    let mut rpc_session = RpcSession::new(rpc, activity);
    rpc_session.update();

    run_app(transformed_entry, entry_file, rpc_session).await?;
    Ok(())
}

async fn run_app(transformed_entry: PathBuf, entry_file: PathBuf, rpc: RpcSession) -> Result<()> {
    let terminal = ratatui::init();
    let mut app = CheckpointTUI::new(
        Some(transformed_entry.to_string_lossy().to_string()),
        Some(entry_file.to_string_lossy().to_string()),
        rpc,
    );
    let result = app.run(terminal).await;
    ratatui::restore();

    result
}

fn copy_runtime_files(ctx: &ProjectContext) -> Result<()> {
    use std::fs;
    use std::process::Command;

    let runtime_src = std::path::Path::new("runtime");
    let runtime_dst = ctx.checkpoint_dir.join("runtime");

    if !runtime_src.exists() {
        return Err(eyre::eyre!("Runtime directory not found at ./runtime"));
    }

    if runtime_dst.exists() {
        fs::remove_dir_all(&runtime_dst)?;
    }

    fs::create_dir_all(&runtime_dst)?;

    copy_dir_selective(runtime_src, &runtime_dst)?;

    println!("[INFO] Runtime files copied to .checkpoint/runtime");

    if runtime_dst.join("package.json").exists() {
        println!("[INFO] Installing runtime dependencies...");
        let install_result = Command::new("bun")
            .arg("install")
            .current_dir(&runtime_dst)
            .output()?;

        if !install_result.status.success() {
            eprintln!("[WARN] Failed to install dependencies, runtime may not work");
        } else {
            println!("[INFO] Dependencies installed successfully");
        }
    }

    Ok(())
}

fn copy_dir_selective(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    use std::fs;

    let skip_items = ["node_modules", ".git", "dist", "build", "target"];

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if skip_items.contains(&file_name_str.as_ref()) {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&file_name);
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_selective(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
