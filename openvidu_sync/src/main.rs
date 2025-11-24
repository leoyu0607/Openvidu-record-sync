use std::{env,fs,io,process::Command,path::Path};
use serde::Deserialize;
use serde_json;
use serde_json::Value;
use log::{debug, info, warn, error};
use simplelog::{CombinedLogger, WriteLogger, ConfigBuilder, LevelFilter};
use time::macros::format_description;
use time::UtcOffset;

#[derive(Deserialize, Debug)]
struct Config {
    mount_dir:  String,
    record_dir: String,
    log_file:  String,
    log_level: String,
    user:      String,
    group:     String,
}
impl Config {
    fn load_from_file() -> anyhow::Result<Self> {
        let exe_path = env::current_exe()?;
        let exe_dir = exe_path.parent().unwrap();
        let config_path = exe_dir.join("Config.json");
        let data = fs::read_to_string(config_path)?;
        let config: Self = serde_json::from_str(&data)?;
        Ok(config)
    }
}

struct RecordInfo {
    //tenant_id: String,
    //chat_id: String,
    dir_name: String,
    status: String,
    synced: bool,
}

fn init_logger(path: &str, level: LevelFilter) {
    let file = fs::OpenOptions::new()
        .create(true)  // 檔案不存在就建立
        .append(true)  // 追加模式，不覆寫
        .open(path)
        .expect("Failed to open log file");

    let log_cfg = ConfigBuilder::new()
        .set_time_offset(UtcOffset::current_local_offset().unwrap())
        .set_time_format_custom(format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
        .build();

    CombinedLogger::init(vec![
        WriteLogger::new(
            level,
            log_cfg,
            file,
        ),
    ])
        .expect("Failed to initialize logger");
}
fn scan_directory(path: &str) -> anyhow::Result<Vec<RecordInfo>> {
    let mut result: Vec<RecordInfo> = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let status_file_name = format!(".recording.{}", name);
            let status_file = fs::read_to_string(entry.path().join(&status_file_name))?;
            let v: Value = serde_json::from_str(&status_file)?;
            let record_status = v["status"].as_str().unwrap_or("").to_string();
            let synced_status = is_synced(&entry.path());
            result.push(RecordInfo { dir_name :name, status:record_status, synced:synced_status } );
        }
    }

    Ok(result)
}

fn write_tag(dir: &str) -> io::Result<()> {
    let tag_file = format!("{}/.synced", dir);
    let tag_tmp = format!("{}/.synced.tmp", dir);
    fs::write(&tag_tmp,"synced")?;
    fs::rename(&tag_tmp, &tag_file)?;
    Ok(())
}

fn is_synced(dir: &Path) -> bool {
    dir.join(".synced").exists()
}

fn record_sync(src: &str, dst: &str) -> anyhow::Result<()> {
    let output = Command::new("rsync")
        .args([
            "-rtv",
            "--no-owner",
            "--no-group",
            "--partial",
            src,
            dst,
        ])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        write_tag(&src)?;
        info!("rsync completed successfully for {}", src);
    }
    if !output.status.success() {
        error!("rsync failed for {}, stderr: {}", src, stderr);
        anyhow::bail!(
            "rsync command failed with status: {}",stderr
        );
    }
    Ok(())
}

fn fix_permissions(user: &str,group: &str,dir: &str) -> anyhow::Result<()> {
    let owner = format!("{}:{}", user, group);
    let output = Command::new("chown")
        .args(["-R", owner.as_str(), dir])
        .output()?;
    if output.status.success() {
        info!("chown to {} succeeded for {}.", owner, dir);
        Ok(())
    }
    else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("chown to {} failed for {}.", owner, dir);
        error!("caused by: {}", stderr);
        anyhow::bail!("chown failed for {}", dir);
    }
}

fn main() {
    let config = Config::load_from_file().unwrap();
    init_logger(config.log_file.as_str(), match config.log_level.to_uppercase().as_str() {
        "DEBUG" => LevelFilter::Debug,
        "INFO" => LevelFilter::Info,
        "WARN" => LevelFilter::Warn,
        "ERROR" => LevelFilter::Error,
        _ => LevelFilter::Info,
    });
    info!("Starting OpenVidu Record Sync Service");
    info!("Sync Path:{}", config.mount_dir);
    let record_dir = config.record_dir.as_str();
    let folders = scan_directory(record_dir).unwrap();
    for d in folders {
        info!("Found record: {}", d.dir_name);
        info!("Record Status: {}", d.status);
        info!("Synced status: {}", d.synced);
        if d.status == "ready" && !d.synced {
            let src_path = format!("{}/{}/", record_dir, d.dir_name);
            let dst_path = format!("{}/{}", config.mount_dir, d.dir_name);
            info!("Syncing from {} to {}", src_path, dst_path);
            match record_sync(&src_path.as_str(), &dst_path.as_str()) {
                Ok(_) => {
                    info!("Sync completed for {}", d.dir_name);
                    if let Err(_e) = fix_permissions(&config.user.as_str(), &config.group.as_str(), &dst_path.as_str()) {}
                }
                Err(e) => error!("Error syncing {}: {}", d.dir_name, e),
            }
        } else if d.status == "ready" && d.synced {
            debug!("Found record: {}", d.dir_name);
            debug!("Record Status: {}", d.status);
            debug!("Synced status: {}", d.synced);
            debug!("Skipping dir {} because this record have been synced!", d.dir_name);
        }
        else {
            warn!("Found record: {}", d.dir_name);
            warn!("Record Status: {}", d.status);
            warn!("Synced status: {}", d.synced);
            warn!("Skipping dir {} with status is {}", d.dir_name, d.status);
        }

    }
}
