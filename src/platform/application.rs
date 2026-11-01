#[cfg(any(target_os = "linux", target_os = "macos"))]
use anyhow::Context;
use anyhow::Result;
use std::sync::{Arc, OnceLock};

pub const APP_ID: &str = "moe.listen.desktop";
pub const APP_NAME: &str = "LISTEN.moe";
pub const ICON_PNG: &[u8] = include_bytes!("../../assets/app-icon.png");

pub fn window_icon() -> Result<Arc<image::RgbaImage>> {
    static ICON: OnceLock<Arc<image::RgbaImage>> = OnceLock::new();
    if let Some(icon) = ICON.get() {
        return Ok(icon.clone());
    }
    let icon = Arc::new(image::load_from_memory(ICON_PNG)?.into_rgba8());
    Ok(ICON.get_or_init(|| icon).clone())
}

pub fn initialize() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let dirs =
            directories::BaseDirs::new().context("Cannot find the desktop data directory")?;
        linux::register(dirs.data_dir(), &std::env::current_exe()?)?;
    }
    #[cfg(target_os = "macos")]
    {
        use objc2::{AllocAnyThread, MainThreadMarker};
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;

        let main_thread =
            MainThreadMarker::new().context("App icon must be set on the main thread")?;
        let data = NSData::with_bytes(ICON_PNG);
        let icon = NSImage::initWithData(NSImage::alloc(), &data)
            .context("Cannot decode the application icon")?;
        // A valid image is supplied, and GPUI initializes AppKit on this thread.
        unsafe {
            NSApplication::sharedApplication(main_thread).setApplicationIconImage(Some(&icon));
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use anyhow::ensure;
    use std::{fs, io::Write, path::Path};

    pub(super) fn register(data_dir: &Path, executable: &Path) -> Result<()> {
        let icon = data_dir.join(format!("icons/hicolor/256x256/apps/{APP_ID}.png"));
        let desktop = data_dir.join(format!("applications/{APP_ID}.desktop"));
        let entry = desktop_entry(executable)?;
        write_if_changed(&icon, ICON_PNG)?;
        write_if_changed(&desktop, entry.as_bytes())?;
        Ok(())
    }

    fn desktop_entry(executable: &Path) -> Result<String> {
        let executable = executable
            .to_str()
            .context("Executable path is not UTF-8")?;
        ensure!(
            !executable.contains(['\n', '\r', '\t', '=', '%']),
            "Executable path cannot be represented in a desktop entry"
        );
        let mut argument = String::new();
        for character in executable.chars() {
            if matches!(character, '\\' | '"' | '`' | '$') {
                argument.push('\\');
            }
            argument.push(character);
        }
        // Desktop values are unescaped before the Exec argument is unquoted.
        let argument = argument.replace('\\', "\\\\");
        Ok(format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={APP_NAME}\n\
             Comment=JPOP and KPOP radio\n\
             Exec=\"{argument}\"\n\
             Icon={APP_ID}\n\
             Terminal=false\n\
             Categories=AudioVideo;Audio;Player;\n\
             StartupWMClass={APP_ID}\n"
        ))
    }

    fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
        match fs::read(path) {
            Ok(existing) if existing == bytes => return Ok(()),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Cannot read desktop integration file"),
        }
        let directory = path.parent().context("Invalid desktop integration path")?;
        fs::create_dir_all(directory)?;
        let mut file = tempfile::NamedTempFile::new_in(directory)?;
        file.write_all(bytes)?;
        file.persist(path)
            .context("Cannot save desktop integration file")?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn registration_matches_wayland_identity_and_updates_the_launcher() -> Result<()> {
            let directory = tempfile::tempdir()?;
            let executable = Path::new("/tmp/LISTEN moe/listen-moe");
            register(directory.path(), executable)?;
            let desktop = directory
                .path()
                .join(format!("applications/{APP_ID}.desktop"));
            let icon = directory
                .path()
                .join(format!("icons/hicolor/256x256/apps/{APP_ID}.png"));
            let entry = fs::read_to_string(&desktop)?;
            assert!(entry.contains(&format!("Icon={APP_ID}\n")));
            assert!(entry.contains(&format!("StartupWMClass={APP_ID}\n")));
            assert!(entry.contains("Exec=\"/tmp/LISTEN moe/listen-moe\"\n"));
            assert_eq!(fs::read(icon)?, ICON_PNG);

            let modified = fs::metadata(&desktop)?.modified()?;
            register(directory.path(), executable)?;
            assert_eq!(fs::metadata(&desktop)?.modified()?, modified);
            register(directory.path(), Path::new("/tmp/new/listen-moe"))?;
            assert!(fs::read_to_string(desktop)?.contains("Exec=\"/tmp/new/listen-moe\"\n"));
            Ok(())
        }

        #[test]
        fn executable_paths_are_quoted_and_cannot_inject_desktop_keys() -> Result<()> {
            let entry = desktop_entry(Path::new("/tmp/a $b`c\"d\\e/listen-moe"))?;
            assert!(entry.contains(r#"Exec="/tmp/a \\$b\\`c\\"d\\\\e/listen-moe""#));
            assert!(desktop_entry(Path::new("/tmp/app\nHidden=true")).is_err());
            assert!(desktop_entry(Path::new("/tmp/app%name")).is_err());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_logo_decodes_and_is_shared_between_windows() -> Result<()> {
        let first = window_icon()?;
        assert_eq!(first.dimensions(), (256, 256));
        assert!(Arc::ptr_eq(&first, &window_icon()?));
        Ok(())
    }
}
