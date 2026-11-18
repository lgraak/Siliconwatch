use siliconwatch_gui::app::Application;
use slint::ComponentHandle;
fn refresh(args: impl IntoIterator<Item = String>) -> Result<Option<u64>, String> {
    let mut args = args.into_iter();
    let mut interval = 1000;
    let mut seen = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--refresh-ms" if !seen => {
                seen = true;
                let v = args.next().ok_or("--refresh-ms requires a value")?;
                if v.starts_with('0') || !v.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(
                        "--refresh-ms must be a canonical integer from 1000 to 60000".into(),
                    );
                }
                interval = v.parse().map_err(|_| "Invalid refresh interval")?;
                if !(1000..=60000).contains(&interval) {
                    return Err("--refresh-ms must be from 1000 to 60000".into());
                }
            }
            _ => return Err(format!("Unknown or duplicate option: {arg}")),
        }
    }
    Ok(Some(interval))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(interval) = refresh(std::env::args().skip(1)).map_err(std::io::Error::other)? else {
        println!(
            "SiliconWatch native read-only sensor client\nUsage: siliconwatch-gui [--refresh-ms 1000..60000]\nStart the daemon separately with --local-api. GUI refresh never triggers acquisition."
        );
        return Ok(());
    };
    let app = Application::new(interval)?;
    let shown = app.borrow().main.show();
    let result = shown.and_then(|()| {
        app.borrow().main.invoke_focus_table();
        slint::run_event_loop_until_quit()
    });
    app.borrow_mut().shutdown();
    result?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_bounded_refresh() {
        assert_eq!(refresh(Vec::new()), Ok(Some(1000)));
        assert_eq!(
            refresh(["--refresh-ms".into(), "60000".into()]),
            Ok(Some(60000))
        );
        for value in ["", "01000", "+1000", "999", "60001", "1000.0"] {
            assert!(refresh(["--refresh-ms".into(), value.into()]).is_err());
        }
        assert!(
            refresh(["--refresh-ms", "1000", "--refresh-ms", "2000"].map(str::to_owned)).is_err()
        );
    }
}
