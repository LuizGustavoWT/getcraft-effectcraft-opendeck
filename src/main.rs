//! OpenDeck actions for the EffectCraft command line interface and its live control bridge.
//!
//! Arguments are stored as a JSON string array and sent directly to the CLI.
//! Deliberately never invoking a shell preserves paths, JSON parameters, and user input exactly.

use std::{path::PathBuf, process::Stdio, time::Duration};

use openaction::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use tokio::process::Command;

const DEFAULT_EXECUTABLE: &str = "effectcraft-cli";
const CATALOG_ARGUMENTS: &[&str] = &["commands", "--json"];
const DEFAULT_ARGUMENTS: &str = r#"["commands","--json"]"#;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_CATALOG_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct CommandSettings {
    /// CLI executable or absolute path. Empty uses the app's conventional CLI name.
    executable: String,
    /// JSON string array, e.g. ["exec", "timeline.play", "{}"].
    arguments: String,
    /// Optional directory in which to run the command.
    working_directory: String,
    /// A user-visible title shown before the optional command output.
    title: String,
    /// Keep stdout/stderr (trimmed) on the Stream Deck key after a successful run.
    show_output: bool,
    /// Wall-clock limit for a process. Zero uses 30 seconds; maximum is 300 seconds.
    timeout_seconds: u64,
}

impl Default for CommandSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            arguments: DEFAULT_ARGUMENTS.to_string(),
            working_directory: String::new(),
            title: String::new(),
            show_output: false,
            timeout_seconds: 30,
        }
    }
}

impl CommandSettings {
    fn executable(&self) -> &str {
        let executable = self.executable.trim();
        if executable.is_empty() {
            DEFAULT_EXECUTABLE
        } else {
            executable
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(if self.timeout_seconds == 0 {
            30
        } else {
            self.timeout_seconds.clamp(1, 300)
        })
    }

    fn title(&self) -> Option<String> {
        (!self.title.trim().is_empty()).then(|| self.title.trim().to_string())
    }
}

#[derive(Debug)]
enum ExecutionError {
    Configuration(String),
    Process(String),
    TimedOut(u64),
    Failed { code: Option<i32>, output: String },
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(message) | Self::Process(message) => f.write_str(message),
            Self::TimedOut(seconds) => write!(f, "Timed out after {seconds} seconds"),
            Self::Failed { code, output } => {
                write!(
                    f,
                    "EffectCraft CLI exited with {}",
                    code.map_or_else(|| "a signal".into(), |code| format!("code {code}"))
                )?;
                if !output.is_empty() {
                    write!(f, ": {output}")?;
                }
                Ok(())
            }
        }
    }
}

fn parse_arguments(raw: &str) -> Result<Vec<String>, ExecutionError> {
    let parsed: Value = serde_json::from_str(raw.trim()).map_err(|error| {
        ExecutionError::Configuration(format!(
            "Arguments must be a JSON array of strings: {error}"
        ))
    })?;
    let values = parsed.as_array().ok_or_else(|| {
        ExecutionError::Configuration("Arguments must be a JSON array of strings.".into())
    })?;
    if values.is_empty() {
        return Err(ExecutionError::Configuration(
            "Add at least one CLI argument, for example [\"commands\", \"--json\"].".into(),
        ));
    }
    if values.len() > MAX_ARGUMENTS {
        return Err(ExecutionError::Configuration(format!(
            "At most {MAX_ARGUMENTS} arguments are allowed."
        )));
    }
    values
        .iter()
        .map(|value| {
            let value = value.as_str().ok_or_else(|| {
                ExecutionError::Configuration(
                    "Every argument must be a string; numbers and JSON values must be quoted."
                        .into(),
                )
            })?;
            if value.len() > MAX_ARGUMENT_BYTES {
                return Err(ExecutionError::Configuration(format!(
                    "An argument is longer than {MAX_ARGUMENT_BYTES} bytes."
                )));
            }
            Ok(value.to_string())
        })
        .collect()
}

fn summary(stdout: &[u8], stderr: &[u8]) -> String {
    let output = if !stderr.is_empty() { stderr } else { stdout };
    let text = String::from_utf8_lossy(output).replace(['\n', '\r'], " ");
    let text = text.trim();
    let mut end = text.len().min(120);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let suffix = if text.len() > end { "…" } else { "" };
    format!("{}{}", &text[..end], suffix)
}

async fn execute(
    settings: &CommandSettings,
    arguments: Vec<String>,
) -> Result<String, ExecutionError> {
    let output = execute_raw(settings, arguments).await?;
    let output_summary = summary(&output.stdout, &output.stderr);
    if output.status.success() {
        Ok(output_summary)
    } else {
        Err(ExecutionError::Failed {
            code: output.status.code(),
            output: output_summary,
        })
    }
}

/// Execute without a shell and retain the full output for a small number of actions that need to
/// read a current property before changing it. The normal action still uses a short summary.
async fn execute_raw(
    settings: &CommandSettings,
    arguments: Vec<String>,
) -> Result<std::process::Output, ExecutionError> {
    let mut command = Command::new(settings.executable());
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if !settings.working_directory.trim().is_empty() {
        command.current_dir(PathBuf::from(settings.working_directory.trim()));
    }
    let seconds = settings.timeout().as_secs();
    let output = tokio::time::timeout(settings.timeout(), command.output())
        .await
        .map_err(|_| ExecutionError::TimedOut(seconds))?
        .map_err(|error| {
            ExecutionError::Process(format!(
                "Could not start {}: {error}",
                settings.executable()
            ))
        })?;
    Ok(output)
}

async fn report(
    instance: &Instance,
    settings: &CommandSettings,
    result: Result<String, ExecutionError>,
    to_inspector: bool,
) {
    match result {
        Ok(output) => {
            let title = if settings.show_output && !output.is_empty() {
                output.clone()
            } else {
                settings.title().unwrap_or_else(|| "Done".to_string())
            };
            let _ = instance.set_title(Some(title), None).await;
            let _ = instance.show_ok().await;
            if to_inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"result", "ok":true, "message": output}),
                    )
                    .await;
            }
        }
        Err(error) => {
            let message = error.to_string();
            log::warn!("EffectCraft CLI action failed: {message}");
            let _ = instance.set_title(Some("Error".to_string()), None).await;
            let _ = instance.show_alert().await;
            if to_inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"result", "ok":false, "message": message}),
                    )
                    .await;
            }
        }
    }
}

async fn run_action(instance: &Instance, settings: &CommandSettings, to_inspector: bool) {
    let result = parse_arguments(&settings.arguments).and_then(|arguments| {
        // Validation above is synchronous; the process work stays on the async runtime.
        Ok(arguments)
    });
    let result = match result {
        Ok(arguments) => execute(settings, arguments).await,
        Err(error) => Err(error),
    };
    report(instance, settings, result, to_inspector).await;
}

fn catalog_items(stdout: &str) -> Vec<Value> {
    let Ok(value) = serde_json::from_str::<Value>(stdout) else {
        return Vec::new();
    };
    let items = value
        .as_array()
        .cloned()
        .or_else(|| value.get("commands").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    items
        .into_iter()
        .filter_map(|item| {
            if let Some(id) = item.as_str() {
                return Some(json!({"id": id, "label": id}));
            }
            let id = ["id", "command", "tool", "name"]
                .into_iter()
                .find_map(|key| item.get(key).and_then(Value::as_str))?;
            let label = item
                .get("label")
                .or_else(|| item.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(id);
            Some(json!({"id": id, "label": label}))
        })
        .collect()
}

async fn send_catalog(instance: &Instance, settings: &CommandSettings) {
    let mut command = Command::new(settings.executable());
    command
        .args(CATALOG_ARGUMENTS)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if !settings.working_directory.trim().is_empty() {
        command.current_dir(PathBuf::from(settings.working_directory.trim()));
    }
    let result = tokio::time::timeout(Duration::from_secs(15), command.output()).await;
    let reply = match result {
        Ok(Ok(output)) if output.status.success() && output.stdout.len() <= MAX_CATALOG_BYTES => {
            let text = String::from_utf8_lossy(&output.stdout);
            let items = catalog_items(&text);
            if items.is_empty() {
                json!({"kind":"catalog", "ok":false, "message":"The installed CLI did not return a JSON command catalog."})
            } else {
                json!({"kind":"catalog", "ok":true, "items":items})
            }
        }
        Ok(Ok(output)) => {
            json!({"kind":"catalog", "ok":false, "message":format!("Could not read the command catalog: {}", summary(&output.stdout, &output.stderr))})
        }
        Ok(Err(error)) => {
            json!({"kind":"catalog", "ok":false, "message":format!("Could not start {}: {error}", settings.executable())})
        }
        Err(_) => {
            json!({"kind":"catalog", "ok":false, "message":"Timed out while loading the command catalog."})
        }
    };
    let _ = instance.send_to_property_inspector(reply).await;
}

struct RunEffectCraftCli;

#[async_trait]
impl Action for RunEffectCraftCli {
    const UUID: &'static str = "wt.luizgustavo.getcraft.effectcraft.run";
    type Settings = CommandSettings;

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        run_action(instance, settings, false).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if let Some(title) = settings.title() {
            let _ = instance.set_title(Some(title), None).await;
        }
        Ok(())
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let effective = payload
            .get("settings")
            .and_then(|value| serde_json::from_value::<CommandSettings>(value.clone()).ok())
            .unwrap_or_else(|| settings.clone());
        match payload.get("request").and_then(Value::as_str) {
            Some("catalog") => send_catalog(instance, &effective).await,
            Some("test") => run_action(instance, &effective, true).await,
            _ => {}
        }
        Ok(())
    }
}

/// Purpose-built controls run through EffectCraft's control bridge, so they operate on the
/// composition that is open on screen instead of creating a disposable headless demo session.
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct EffectControlSettings {
    executable: String,
    bridge_port: String,
    working_directory: String,
    control: String,
    text: String,
    color: String,
    effect: String,
    title: String,
    timeout_seconds: u64,
}

impl Default for EffectControlSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            bridge_port: "9877".into(),
            working_directory: String::new(),
            control: "playback".into(),
            text: "New Text".into(),
            color: "#4f8cff".into(),
            effect: "Gaussian Blur".into(),
            title: String::new(),
            timeout_seconds: 15,
        }
    }
}

impl EffectControlSettings {
    fn command_settings(&self) -> CommandSettings {
        CommandSettings {
            executable: self.executable.clone(),
            arguments: DEFAULT_ARGUMENTS.into(),
            working_directory: self.working_directory.clone(),
            title: self.title.clone(),
            show_output: false,
            timeout_seconds: self.timeout_seconds,
        }
    }

    fn bridge_port(&self) -> Result<String, ExecutionError> {
        let port = self.bridge_port.trim().parse::<u16>().map_err(|_| {
            ExecutionError::Configuration("Bridge port must be a number from 1 to 65535.".into())
        })?;
        if port == 0 {
            return Err(ExecutionError::Configuration(
                "Bridge port must be a number from 1 to 65535.".into(),
            ));
        }
        Ok(port.to_string())
    }

    fn title_or(&self, fallback: &str) -> String {
        if self.title.trim().is_empty() {
            fallback.into()
        } else {
            self.title.trim().into()
        }
    }
}

/// A control and the exact engine command it maps to. These are deliberately small, repeatable
/// operations suitable for physical keys, rather than a second generic CLI form.
fn effect_control_command(
    settings: &EffectControlSettings,
) -> Result<(String, Value, String), ExecutionError> {
    let text = if settings.text.trim().is_empty() {
        "New Text"
    } else {
        settings.text.trim()
    };
    let color = if settings.color.trim().is_empty() {
        "#4f8cff"
    } else {
        settings.color.trim()
    };
    let effect = if settings.effect.trim().is_empty() {
        "Gaussian Blur"
    } else {
        settings.effect.trim()
    };
    let (command, params, title) = match settings.control.as_str() {
        "playback" => ("playback.toggle", json!({}), "Play / Stop"),
        "previousFrame" => ("time.previousFrame", json!({}), "Previous Frame"),
        "nextFrame" => ("time.nextFrame", json!({}), "Next Frame"),
        "previousKey" => ("time.previousKey", json!({}), "Previous Key"),
        "nextKey" => ("time.nextKey", json!({}), "Next Key"),
        "splitLayer" => ("edit.splitLayer", json!({}), "Split Layer"),
        "addText" => ("layer.newText", json!({"text": text}), "Add Text"),
        "addSolid" => ("layer.newSolid", json!({"color": color}), "Add Solid"),
        "addShape" => ("layer.newShape", json!({"kind":"rect"}), "Add Shape"),
        "addAdjustment" => ("layer.newAdjustment", json!({}), "Adjustment Layer"),
        "opacityKey" => (
            "keys.toggleTransform",
            json!({"prop":"opacity"}),
            "Opacity Key",
        ),
        "rotationKey" => (
            "keys.toggleTransform",
            json!({"prop":"rotation"}),
            "Rotation Key",
        ),
        "applyEffect" => ("effect.apply", json!({"effect": effect}), "Apply Effect"),
        "showRenderQueue" => (
            "window.panel",
            json!({"panel":"renderQueue"}),
            "Render Queue",
        ),
        "renderQueue" => ("renderQueue.render", json!({"wait":false}), "Render Queue"),
        "stopRender" => ("renderQueue.stop", json!({}), "Stop Render"),
        "undo" => ("edit.undo", json!({}), "Undo"),
        "redo" => ("edit.redo", json!({}), "Redo"),
        other => {
            return Err(ExecutionError::Configuration(format!(
                "Unknown EffectCraft control `{other}`. Choose a control in the Property Inspector."
            )));
        }
    };
    Ok((command.into(), params, settings.title_or(title)))
}

fn effect_exec_arguments(
    settings: &EffectControlSettings,
    command: &str,
    params: Value,
) -> Result<Vec<String>, ExecutionError> {
    Ok(vec![
        "exec".into(),
        command.into(),
        "--params".into(),
        serde_json::to_string(&params)
            .map_err(|error| ExecutionError::Configuration(error.to_string()))?,
        "--bridge".into(),
        settings.bridge_port()?,
        "--json".into(),
    ])
}

async fn report_live(instance: &Instance, title: String, result: Result<String, ExecutionError>) {
    match result {
        Ok(_) => {
            let _ = instance.set_title(Some(title), None).await;
            let _ = instance.show_ok().await;
        }
        Err(error) => {
            log::warn!("EffectCraft live control failed: {error}");
            let _ = instance.set_title(Some("Error"), None).await;
            let _ = instance.show_alert().await;
        }
    }
}

struct EffectCraftControl;

#[async_trait]
impl Action for EffectCraftControl {
    const UUID: &'static str = "wt.luizgustavo.getcraft.effectcraft.control";
    type Settings = EffectControlSettings;

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let result = match effect_control_command(settings) {
            Ok((command, params, title)) => {
                let args = effect_exec_arguments(settings, &command, params);
                match args {
                    Ok(args) => {
                        let output = execute(&settings.command_settings(), args).await;
                        report_live(instance, title, output).await;
                        return Ok(());
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };
        report_live(instance, "Error".into(), result).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if let Ok((_, _, title)) = effect_control_command(settings) {
            let _ = instance.set_title(Some(title), None).await;
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct EffectDialSettings {
    executable: String,
    bridge_port: String,
    working_directory: String,
    property: String,
    custom_path: String,
    step: f64,
    title: String,
    timeout_seconds: u64,
}

impl Default for EffectDialSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            bridge_port: "9877".into(),
            working_directory: String::new(),
            property: "opacity".into(),
            custom_path: String::new(),
            step: 1.0,
            title: String::new(),
            timeout_seconds: 15,
        }
    }
}

impl EffectDialSettings {
    fn as_control_settings(&self) -> EffectControlSettings {
        EffectControlSettings {
            executable: self.executable.clone(),
            bridge_port: self.bridge_port.clone(),
            working_directory: self.working_directory.clone(),
            title: self.title.clone(),
            timeout_seconds: self.timeout_seconds,
            ..Default::default()
        }
    }

    fn property_path(&self) -> Result<(&str, &str), ExecutionError> {
        match self.property.as_str() {
            "opacity" => Ok(("transform/opacity", "Opacity")),
            "rotation" => Ok(("transform/rotation", "Rotation")),
            "scale" => Ok(("transform/scale", "Scale")),
            "custom" if !self.custom_path.trim().is_empty() => {
                Ok((self.custom_path.trim(), "Property"))
            }
            "custom" => Err(ExecutionError::Configuration(
                "Enter a property path for a Custom dial.".into(),
            )),
            other => Err(ExecutionError::Configuration(format!(
                "Unknown dial property `{other}`."
            ))),
        }
    }

    fn step(&self) -> Result<f64, ExecutionError> {
        if !self.step.is_finite() || self.step == 0.0 {
            return Err(ExecutionError::Configuration(
                "Dial step must be a non-zero number.".into(),
            ));
        }
        Ok(self.step.clamp(-1_000_000.0, 1_000_000.0))
    }
}

fn effect_output_json(output: std::process::Output) -> Result<Value, ExecutionError> {
    if !output.status.success() {
        return Err(ExecutionError::Failed {
            code: output.status.code(),
            output: summary(&output.stdout, &output.stderr),
        });
    }
    serde_json::from_slice(&output.stdout).map_err(|error| {
        ExecutionError::Process(format!("EffectCraft returned invalid JSON: {error}"))
    })
}

fn adjust_effect_value(value: &Value, delta: f64) -> Result<Value, ExecutionError> {
    if let Some(value) = value.as_f64() {
        return Ok(json!(value + delta));
    }
    if let Some(values) = value.as_array() {
        let adjusted: Option<Vec<f64>> = values
            .iter()
            .map(Value::as_f64)
            .map(|v| v.map(|n| n + delta))
            .collect();
        return adjusted.map(|v| json!(v)).ok_or_else(|| {
            ExecutionError::Configuration(
                "The selected property is not numeric, so it cannot be adjusted with a dial."
                    .into(),
            )
        });
    }
    Err(ExecutionError::Configuration(
        "The selected property is not numeric, so it cannot be adjusted with a dial.".into(),
    ))
}

async fn adjust_effect_property(
    settings: &EffectDialSettings,
    ticks: i16,
) -> Result<(String, String), ExecutionError> {
    let (path, label) = settings.property_path()?;
    let controls = settings.as_control_settings();
    let port = controls.bridge_port()?;
    let command = controls.command_settings();
    let get = vec![
        "get".into(),
        "-".into(),
        "-".into(),
        path.into(),
        "--bridge".into(),
        port.clone(),
        "--json".into(),
    ];
    let current = effect_output_json(execute_raw(&command, get).await?)?;
    let value = current.get("value").ok_or_else(|| {
        ExecutionError::Process("EffectCraft did not return a property value.".into())
    })?;
    let next = adjust_effect_value(value, settings.step()? * f64::from(ticks))?;
    let next_argument = serde_json::to_string(&next)
        .map_err(|error| ExecutionError::Configuration(error.to_string()))?;
    let set = vec![
        "set".into(),
        "-".into(),
        "-".into(),
        path.into(),
        next_argument,
        "--bridge".into(),
        port,
        "--json".into(),
    ];
    execute(&command, set).await?;
    let shown = match &next {
        Value::Number(value) => value.to_string(),
        Value::Array(_) => "adjusted".into(),
        _ => "adjusted".into(),
    };
    Ok((label.into(), shown))
}

struct EffectCraftDial;

#[async_trait]
impl Action for EffectCraftDial {
    const UUID: &'static str = "wt.luizgustavo.getcraft.effectcraft.dial";
    type Settings = EffectDialSettings;

    async fn dial_rotate(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        if ticks == 0 {
            return Ok(());
        }
        match adjust_effect_property(settings, ticks).await {
            Ok((label, value)) => {
                let title = if settings.title.trim().is_empty() {
                    format!("{label}\n{value}")
                } else {
                    settings.title.trim().into()
                };
                let _ = instance.set_title(Some(title), None).await;
                let _ = instance.show_ok().await;
            }
            Err(error) => {
                log::warn!("EffectCraft dial failed: {error}");
                let _ = instance.set_title(Some("Error"), None).await;
                let _ = instance.show_alert().await;
            }
        }
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let controls = settings.as_control_settings();
        let result = (|| -> Result<(Vec<String>, String), ExecutionError> {
            let (path, label) = settings.property_path()?;
            Ok((
                effect_exec_arguments(&controls, "prop.toggleKey", json!({"path":path}))?,
                format!("{label} Key"),
            ))
        })();
        match result {
            Ok((args, title)) => {
                report_live(
                    instance,
                    title,
                    execute(&controls.command_settings(), args).await,
                )
                .await
            }
            Err(error) => report_live(instance, "Error".into(), Err(error)).await,
        }
        Ok(())
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if let Ok((_, label)) = settings.property_path() {
            let title: String = if settings.title.trim().is_empty() {
                label.into()
            } else {
                settings.title.trim().into()
            };
            let _ = instance.set_title(Some(title), None).await;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Never,
    );
    register_action(RunEffectCraftCli).await;
    register_action(EffectCraftControl).await;
    register_action(EffectCraftDial).await;
    run(std::env::args().collect()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_string_argument_vector() {
        assert_eq!(
            parse_arguments(r#"["exec","timeline.play","{}"]"#).unwrap(),
            ["exec", "timeline.play", "{}"]
        );
    }

    #[test]
    fn rejects_non_string_arguments() {
        assert!(parse_arguments(r#"["exec", 4]"#).is_err());
    }

    #[test]
    fn stderr_is_preferred_in_a_failure_summary() {
        assert_eq!(summary(b"normal", b"bad\nnews"), "bad news");
    }

    #[test]
    fn has_a_usable_default_command() {
        assert_eq!(
            parse_arguments(&CommandSettings::default().arguments).unwrap(),
            CATALOG_ARGUMENTS
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn honors_a_one_second_timeout() {
        assert_eq!(
            CommandSettings {
                timeout_seconds: 1,
                ..Default::default()
            }
            .timeout(),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn live_control_uses_the_open_project_bridge() {
        let settings = EffectControlSettings {
            control: "splitLayer".into(),
            bridge_port: "9877".into(),
            ..Default::default()
        };
        let (command, params, _) = effect_control_command(&settings).unwrap();
        assert_eq!(command, "edit.splitLayer");
        assert_eq!(params, json!({}));
        let arguments = effect_exec_arguments(&settings, &command, params).unwrap();
        assert_eq!(arguments[0], "exec");
        assert!(arguments
            .windows(2)
            .any(|pair| pair == ["--bridge", "9877"]));
    }

    #[test]
    fn property_dial_adjusts_scalars_and_vectors() {
        assert_eq!(adjust_effect_value(&json!(25), 1.5).unwrap(), json!(26.5));
        assert_eq!(
            adjust_effect_value(&json!([100, 100, 100]), -2.0).unwrap(),
            json!([98.0, 98.0, 98.0])
        );
        assert!(adjust_effect_value(&json!("not a number"), 1.0).is_err());
    }
}
