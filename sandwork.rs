use std::{ fs, env };
use std::path::{ Path, PathBuf };
use std::os::unix::process::CommandExt;
use std::process::Command;
use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    overlay: Vec<PathBuf>,
    shadow: Vec<PathBuf>,
    robind: Vec<PathBuf>,
    rwbind: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let home = env::home_dir().context("not found home")?;
    let sandwork = home.join(".sandwork");
    let rwsrc = sandwork.join("rwsrc");
    let workdir = sandwork.join("workdir");
    let empty = sandwork.join("empty");

    fs::create_dir_all(&empty)?;

    let config: Config = {
        let buf = fs::read_to_string(sandwork.join("config.toml"))
            .context("read config failed")?;
        toml::from_str(&buf).context("parse config failed")?
    };
    
    let mut cmd = Command::new("bwrap");
    cmd
        .args(&[
            "--die-with-parent",
            "--unshare-all",
            "--share-net",
            "--dev", "/dev",
            "--proc", "/proc",
            "--tmpfs", "/tmp",
            "--ro-bind", "/usr", "/usr",
            "--ro-bind", "/etc", "/etc",
            "--ro-bind", "/opt", "/opt",
            "--symlink", "/usr/bin", "/bin",
            "--symlink", "/usr/bin", "/sbin",
            "--symlink", "/usr/lib", "/lib",
            "--symlink", "/usr/lib", "/lib64",
            "--ro-bind", "/dev/null", "/etc/shadow",
            "--ro-bind", "/dev/null", "/etc/shadow-",
        ]);

    if let (Some(xdg_rt), Some(display)) =
        (env::var_os("XDG_RUNTIME_DIR"), env::var_os("WAYLAND_DISPLAY"))
    {
        let path = PathBuf::from(xdg_rt).join(display);
        cmd.arg("--ro-bind").arg(&path).arg(&path);
    }

    for dir in &config.overlay {
        let path = home.join(dir);
        let rwsrc = rwsrc.join(dir);
        let workdir = workdir.join(dir);

        fs::create_dir_all(&rwsrc)?;
        fs::create_dir_all(&workdir)?;        
        
        cmd
            .arg("--overlay-src").arg(&path)
            .arg("--overlay")
            .arg(&rwsrc)
            .arg(&workdir)
            .arg(&path);
    }

    for bind in &config.rwbind {
        let buf;
        let path;

        if bind.is_relative() {
            buf = home.join(bind);
            path = &buf;
        } else {
            path = bind;
        };

        cmd.arg("--bind-try").arg(path).arg(path);        
    }    

    for bind in &config.robind {
        let buf;
        let path;

        if bind.is_relative() {
            buf = home.join(bind);
            path = &buf;
        } else {
            path = bind;
        };

        cmd.arg("--ro-bind-try").arg(path).arg(path);        
    }

    for shadow in &config.shadow {
        let buf;
        let path;

        if shadow.is_relative() {
            buf = home.join(shadow);
            path = &buf;
        } else {
            path = shadow;
        };
        let src = if path.is_file() {
            Path::new("/dev/null")            
        } else {
            &empty
        };

        cmd.arg("--ro-bind-try").arg(src).arg(path);
    }    

    cmd
        .arg("--")
        .arg("/usr/bin/bash");

    Err(cmd.exec().into())
}
