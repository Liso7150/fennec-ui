//! Shared composer controls + STT transcription used by both Fennec shells.
//! Anything two repos need lives here, not duplicated per app.

use gpui_kit::component::{
    button::{Button, ButtonVariants},
    label::Label,
    Sizable, Theme,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// Human-readable model name for a model id.
pub fn model_display_name(model: &str) -> String {
    match model {
        "qwen3_5_moe" => "Qwen3.5-MoE".to_string(),
        "edge0_145b" => "Edge0-145B".to_string(),
        "qwen36_35b" => "Qwen36-35B".to_string(),
        _ => model.to_string(),
    }
}

pub fn access_icon(mode: &str) -> &'static str {
    match mode {
        "auto" => "🕐",
        "full" => "⚠",
        _ => "🛡",
    }
}

pub fn access_label(mode: &str) -> &'static str {
    match mode {
        "auto" => "Auto approve",
        "full" => "Full access",
        _ => "Ask every time",
    }
}

pub fn access_color(mode: &str, t: &Theme) -> Hsla {
    match mode {
        "auto" => t.success,
        "full" => t.red,
        _ => t.primary,
    }
}

/// The permission-mode dropdown (ask / auto / full), rendered at the window
/// root by the caller. `on_select(mode)` applies the choice.
pub fn permission_dropdown<F>(t: &Theme, current: &str, on_select: F) -> Div
where
    F: Fn(String, &mut Window, &mut App) + Clone + 'static,
{
    let mut d = div().absolute().bottom(px(70.0)).left(px(20.0))
        .rounded_md().border_1().border_color(t.border).bg(t.background).shadow_lg()
        .p_2().w(px(460.0)).flex().flex_col().gap_1();
    for (mode, icon, title, desc) in [
        ("ask", "🛡", "Ask every time", "Always ask before commands, external edits, and internet use"),
        ("auto", "🕐", "Auto approve", "Only asks when a potential risk is detected."),
        ("full", "⚠", "Full access", "Never ask, with full access to your files, terminal, and the internet"),
    ] {
        let selected = current == mode;
        let danger = mode == "full";
        let cb = on_select.clone();
        let mode_owned = mode.to_string();
        d = d.child(
            Button::new(format!("perm-opt-{mode}")).ghost().w_full().justify_start()
                .on_click(move |_, window, cx| cb(mode_owned.clone(), window, cx))
                .child(div().flex().items_start().gap_3().px_3().py_3().w_full()
                    .child(Label::new(icon).text_size(px(18.0)).mt(px(2.0)))
                    .child(div().flex().flex_col().flex_1().gap_0p5()
                        .child(Label::new(title).text_size(px(13.0)).font_weight(FontWeight::MEDIUM)
                            .text_color(if danger { t.red } else { t.foreground }))
                        .child(Label::new(desc).text_size(px(11.0)).text_color(t.muted_foreground)))
                    .child(Label::new(if selected { "✓" } else { "" }).text_size(px(13.0)).text_color(t.success).mt(px(2.0)))),
        );
    }
    d
}

/// The model-selector dropdown with a "Download new model…" row.
pub fn model_dropdown<F, G>(t: &Theme, current: &str, on_select: F, on_download: G) -> Div
where
    F: Fn(String, &mut Window, &mut App) + Clone + 'static,
    G: Fn(&mut Window, &mut App) + 'static,
{
    let mut d = div().absolute().bottom(px(70.0)).right(px(20.0))
        .rounded_md().border_1().border_color(t.border).bg(t.background).shadow_lg()
        .p_2().w(px(220.0)).flex().flex_col().gap_1();
    for (id, name) in [("qwen3_5_moe", "Qwen3.5-MoE"), ("edge0_145b", "Edge0-145B"), ("qwen36_35b", "Qwen36-35B")] {
        let selected = current == id;
        let cb = on_select.clone();
        let id_owned = id.to_string();
        d = d.child(
            Button::new(format!("model-{id}")).ghost().small().w_full().justify_start()
                .child(div().flex().items_center().gap_3().px_3().py_2()
                    .child(Label::new(name).text_size(px(13.0)).font_weight(FontWeight::MEDIUM))
                    .child(div().flex_1())
                    .child(Label::new(if selected { "✓" } else { "" }).text_size(px(13.0)).text_color(t.success)))
                .on_click(move |_, window, cx| cb(id_owned.clone(), window, cx)),
        );
    }
    d.child(div().border_t_1().border_color(t.border).my_1())
        .child(Button::new("model-download").ghost().small().w_full().justify_start()
            .child(div().flex().items_center().gap_3().px_3().py_2()
                .child(Label::new("⬇").text_size(px(15.0)))
                .child(Label::new("Download new model...").text_size(px(12.0)).text_color(t.muted_foreground)))
            .on_click(move |_, window, cx| on_download(window, cx)))
}

/// Write captured mono f32 samples to a WAV and POST to the faster-whisper
/// sidecar (port 8183). Tries the OpenAI-compatible route then /transcribe.
pub fn transcribe_samples(samples: &[f32], rate: u32) -> String {
    if samples.is_empty() {
        return String::new();
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav = Vec::new();
    {
        let mut writer = match hound::WavWriter::new(std::io::Cursor::new(&mut wav), spec) {
            Ok(w) => w,
            Err(_) => return String::new(),
        };
        for &s in samples {
            let _ = writer.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16);
        }
        let _ = writer.finalize();
    }
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    // The Fennec sidecar (fennec_core.stt) takes a RAW audio body on /transcribe.
    if let Ok(resp) = client
        .post("http://127.0.0.1:8183/transcribe")
        .header("Content-Type", "audio/wav")
        .body(wav.clone())
        .send()
    {
        if let Ok(json) = resp.json::<serde_json::Value>() {
            if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                return text.to_string();
            }
        }
    }
    // OpenAI-compatible fallback (multipart).
    for path in ["/v1/audio/transcriptions", "/transcribe"] {
        let url = format!("http://127.0.0.1:8183{}", path);
        let part = match reqwest::blocking::multipart::Part::bytes(wav.clone())
            .file_name("audio.wav")
            .mime_str("audio/wav")
        {
            Ok(p) => p,
            Err(_) => continue,
        };
        let form = reqwest::blocking::multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-1");
        if let Ok(resp) = client.post(&url).multipart(form).send() {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                    return text.to_string();
                }
            }
        }
    }
    String::new()
}
