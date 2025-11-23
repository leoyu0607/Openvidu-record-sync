use std::{env,fs,process::Command,path::Path};
use serde::Deserialize;
use serde_json;
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct Config {
    mount_dir:  String,
    record_dir: String,
}
impl Config {
    fn load_from_file() -> anyhow::Result<Self> {
        let exe_path = env::current_exe()?;
        //println!("exe path:{:?}", exe_path);
        let exe_dir = exe_path.parent().unwrap();
        //println!("exe_dir:{:?}", exe_dir);
        let config_path = exe_dir.join("Config.json");
        //print!("config_path:{:?}\n", config_path);
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

fn scan_directory(path: &str) -> anyhow::Result<Vec<RecordInfo>> {
    let mut result: Vec<RecordInfo> = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let status_file_name = format!(".recording.{}", name);
            let status_file = fs::read_to_string(entry.path().join(&status_file_name))?;
            let record_status = match fs::read_to_string(&status_file) {
                Ok(status_file) => {
                    match serde_json::from_str::<Value>(&status_file) {
                        Ok(v) => v["status"].as_str().unwrap_or("").to_string(),
                        Err(e) => {
                            eprintln!("JSON parse error in {:?}: {}", status_file, e);
                            continue; // 這個目錄跳過，整體繼續
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read status file {:?}: {}", status_file, e);
                    continue;
                }
            };
            let synced_status = is_synced(&entry.path().to_string_lossy());
            result.push(RecordInfo { dir_name :name, status:record_status, synced:synced_status } );
        }
    }

    Ok(result)
}

fn write_tag(dir: &str) -> std::io::Result<()> {
    let tag_file = format!("{}/.synced", dir);
    let tag_tmp = format!("{}/.synced.tmp", dir);
    fs::write(&tag_tmp,"synced")?;
    fs::rename(&tag_tmp, &tag_file)?;
    Ok(())
}

fn is_synced(dir: &str) -> bool {
    Path::new(dir).join(".synced").exists()
}

fn record_sync(src: &str, dst: &str) -> anyhow::Result<()> {
    let output = Command::new("rsync")
        .args([
            "-avh",
            "--partial",
            &src,
            &dst,
        ])
        .output()?;
    if output.status.success() {write_tag(&src)?;}
    if !output.status.success() {
        anyhow::bail!(
            "rsync command failed with status: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn main() {
    let config = Config::load_from_file().unwrap();
    println!("Mount Path:{}", config.mount_dir);
    let record_dir = &config.record_dir;
    let folders = scan_directory(record_dir).unwrap();
    for d in folders {
        println!("Found dir:{}", d.dir_name);
        println!("Record Status: {}", d.status);
        println!("Synced status:{}", d.synced);
        if d.status == "ready" && !d.synced {
            let src_path = format!("{}/{}/", record_dir, d.dir_name);
            let dst_path = format!("{}/{}", config.mount_dir, d.dir_name);
            println!("Syncing from {} to {}", src_path, dst_path);
            match record_sync(&src_path, &dst_path) {
                Ok(_) => println!("Sync completed for {}", d.dir_name),
                Err(e) => eprintln!("Error syncing {}: {}", d.dir_name, e),
            }
        } else if d.status == "ready" && d.synced {
            println!("Skipping dir {} because this record have been synced!", d.dir_name);
        }
        else { println!("Skipping dir {} with status is {}", d.dir_name, d.status); }
    }
}
