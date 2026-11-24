//! Shared composer controls + STT transcription for the Fennec desktop shell.
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

/// The PWA's permission trigger paints a mode-colored bullet (`•`), not a
/// glyph per mode; the color alone says which mode is on.
pub fn access_icon(_mode: &str) -> &'static str {
    "•"
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
        ("ask", "•", "Ask every time", "Always ask before commands, external edits, and internet use"),
        ("auto", "•", "Auto approve", "Only asks when a potential risk is detected."),
        ("full", "•", "Full access", "Never ask, with full access to your files, terminal, and the internet"),
    ] {
        let selected = current == mode;
        let danger = mode == "full";
        let cb = on_select.clone();
        let mode_owned = mode.to_string();
        d = d.child(
            Button::new(format!("perm-opt-{mode}")).ghost().w_full().justify_start()
                .on_click(move |_, window, cx| cb(mode_owned.clone(), window, cx))
                .child(div().flex().items_start().gap_3().px_3().py_3().w_full()
                    .child(Label::new(icon).text_size(px(18.0)).mt(px(2.0)).text_color(access_color(mode, t)))
                    .child(div().flex().flex_col().flex_1().gap_0p5()
                        .child(Label::new(title).text_size(px(13.0)).font_weight(FontWeight::MEDIUM)
                            .text_color(if danger { t.red } else { t.foreground }))
                        .child(Label::new(desc).text_size(px(11.0)).text_color(t.muted_foreground)))
                    .child(Label::new(if selected { "✓" } else { "" }).text_size(px(13.0)).text_color(t.success).mt(px(2.0)))),
        );
    }
    d
}

/// The reasoning levels the model dropdown offers, in the order it draws them.
/// The core owns what each one means to the engine; this is only the vocabulary
/// the shells draw, so the two surfaces cannot spell the same level differently.
pub const REASONING_LEVELS: [(&str, &str); 4] = [
    ("auto", "Auto"),
    ("low", "Low"),
    ("medium", "Medium"),
    ("high", "High"),
];

/// The model-selector dropdown with a "Download new model…" row and the
/// reasoning-effort rows the same dropdown carries. `served` is the checkpoint
/// the engine proved it holds: it leads the list so a model nobody is running
/// cannot be the only one checked.
pub fn model_dropdown<F, G, H>(
    t: &Theme,
    current: &str,
    served: Option<&str>,
    on_select: F,
    on_download: G,
    effort: &str,
    on_effort: H,
) -> Div
where
    F: Fn(String, &mut Window, &mut App) + Clone + 'static,
    G: Fn(&mut Window, &mut App) + 'static,
    H: Fn(String, &mut Window, &mut App) + Clone + 'static,
{
    let mut d = div().absolute().bottom(px(70.0)).right(px(20.0))
        .rounded_md().border_1().border_color(t.border).bg(t.background).shadow_lg()
        .p_2().w(px(220.0)).flex().flex_col().gap_1();
    let mut rows: Vec<(String, String)> = Vec::new();
    if let Some(id) = served {
        rows.push((id.to_string(), model_display_name(id)));
    }
    for (id, name) in [("qwen3_5_moe", "Qwen3.5-MoE"), ("edge0_145b", "Edge0-145B"), ("qwen36_35b", "Qwen36-35B")] {
        if !rows.iter().any(|(row, _)| row == id) {
            rows.push((id.to_string(), name.to_string()));
        }
    }
    for (id, name) in rows {
        let selected = current == id;
        let cb = on_select.clone();
        let id_owned = id.clone();
        d = d.child(
            Button::new(format!("model-{id}")).ghost().small().w_full().justify_start()
                .child(div().flex().items_center().gap_3().px_3().py_2()
                    .child(Label::new(name).text_size(px(13.0)).font_weight(FontWeight::MEDIUM))
                    .child(div().flex_1())
                    .child(Label::new(if selected { "✓" } else { "" }).text_size(px(13.0)).text_color(t.success)))
                .on_click(move |_, window, cx| cb(id_owned.clone(), window, cx)),
        );
    }
    d = d.child(div().border_t_1().border_color(t.border).my_1())
        .child(Button::new("model-download").ghost().small().w_full().justify_start()
            .child(div().flex().items_center().gap_3().px_3().py_2()
                .child(Label::new("⬇").text_size(px(15.0)))
                .child(Label::new("Download new model...").text_size(px(12.0)).text_color(t.muted_foreground)))
            .on_click(move |_, window, cx| on_download(window, cx)));
    d = d.child(div().border_t_1().border_color(t.border).mt_1()
        .child(Label::new("Reasoning").text_size(px(10.0))
            .text_color(t.muted_foreground).px_3().pt_2()));
    for (level, name) in REASONING_LEVELS {
        let selected = effort == level;
        let cb = on_effort.clone();
        let level_owned = level.to_string();
        d = d.child(
            Button::new(format!("effort-{level}")).ghost().small().w_full().justify_start()
                .child(div().flex().items_center().gap_3().px_3().py_2()
                    .child(Label::new(name).text_size(px(13.0)))
                    .child(div().flex_1())
                    .child(Label::new(if selected { "✓" } else { "" }).text_size(px(13.0)).text_color(t.success)))
                .on_click(move |_, window, cx| cb(level_owned.clone(), window, cx)),
        );
    }
    d
}

/// Write captured mono f32 samples to a WAV and hand them to the transcriber.
/// `server.py:CLIENT_HEADER` — core answers 403 on every `/api/*` without it.
const CLIENT_HEADER: &str = "X-Fennec-Client";

/// The sidecar's own port. Core supervises it, so this is only the fallback for
/// one someone started by hand.
const SIDECAR: &str = "http://127.0.0.1:8183";

/// Render the captured mono PCM as a 16-bit WAV.
fn encode_wav(samples: &[f32], rate: u32) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav = Vec::new();
    {
        let mut writer = hound::WavWriter::new(std::io::Cursor::new(&mut wav), spec)
            .map_err(|e| format!("could not write WAV: {e}"))?;
        for &s in samples {
            let _ = writer.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16);
        }
        writer.finalize().map_err(|e| format!("could not finish WAV: {e}"))?;
    }
    Ok(wav)
}

fn http(timeout: std::time::Duration) -> Result<reqwest::blocking::Client, String> {
    let mut builder = reqwest::blocking::Client::builder().timeout(timeout);
    if let Ok(token) = std::env::var("FENNEC_APP_TOKEN") {
        if !token.is_empty() {
            builder = builder.default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                if let Ok(v) = format!("Bearer {token}").parse() {
                    h.insert(reqwest::header::AUTHORIZATION, v);
                }
                h
            });
        }
    }
    builder.build().map_err(|e| format!("could not build the client: {e}"))
}

/// Pull `text` out of a transcription reply.
fn transcript(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body).ok()?
        .get("text")?.as_str().map(|t| t.to_string())
}

/// The sentence inside a refusal body.
///
/// Both servers answer a bad clip with `{"error": "..."}` — the agent's route
/// and core's both — so the toast says "audio must be at most 16 MiB" instead of
/// a line of JSON the user has to parse.
fn refusal(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body).ok()
        .and_then(|v| v.get("error").or_else(|| v.get("detail")).and_then(|e| e.as_str()).map(str::to_string))
        .unwrap_or_else(|| body.trim().to_string())
}

/// Turn the recorded samples into text.
///
/// The clip goes to `POST /api/stt`, which is the route that starts the
/// faster-whisper sidecar on demand — the Rust agent owns it now
/// (`fennec_client::Speech`), and core serves the same contract. The direct
/// `SIDECAR` calls after it are only for a sidecar someone launched by hand.
/// Going to the sidecar first and never to the server is what made dictation
/// dead on arrival: nothing owns the port, every request is refused, and the
/// caller got back an empty string it could not tell apart from a silent
/// microphone.
///
/// `Ok` holds the transcript and is empty when the clip carried no speech;
/// `Err` is what each lane said, which is the difference between "you said
/// nothing" and "there is nothing to hear you".
pub fn transcribe_samples(samples: &[f32], rate: u32, core_base: &str) -> Result<String, String> {
    if samples.is_empty() {
        return Err("the microphone captured nothing".into());
    }
    let wav = encode_wav(samples, rate)?;
    let mut failures = Vec::new();

    if !core_base.is_empty() {
        let url = format!("{}/api/stt", core_base.trim_end_matches('/'));
        match http(std::time::Duration::from_secs(300))
            .and_then(|client| client.post(&url)
                .header(CLIENT_HEADER, "1")
                .header(reqwest::header::CONTENT_TYPE, "audio/wav")
                .body(wav.clone())
                .send()
                .map_err(|e| format!("{e}")))
        {
            Ok(resp) => {
                let status = resp.status();
                match resp.text() {
                    // A cold sidecar is started and loaded inside this request,
                    // so a slow first answer is expected and 200 is the proof.
                    Ok(body) if status.is_success() => match transcript(&body) {
                        Some(text) => return Ok(text),
                        None => return Err(format!("{url} replied without a text field: {body}")),
                    },
                    // 405, not 404, is what an agent without the route answers:
                    // the static-asset fallback only takes GET and HEAD.
                    Ok(_) if matches!(status.as_u16(), 404 | 405) => {
                        failures.push(format!("{url}: this service has no /api/stt, update the server"));
                    }
                    Ok(body) => failures.push(format!("{url}: http {status}: {}", refusal(&body))),
                    Err(e) => failures.push(format!("{url}: unreadable reply: {e}")),
                }
            }
            Err(e) => failures.push(format!("{url}: {e}")),
        }
    }

    let client = http(std::time::Duration::from_secs(60))?;
    // `stt.py` takes the raw bytes, not a multipart form.
    match client.post(format!("{SIDECAR}/transcribe"))
        .header(reqwest::header::CONTENT_TYPE, "audio/wav")
        .body(wav.clone())
        .send()
    {
        Ok(resp) => {
            let status = resp.status();
            match resp.text() {
                Ok(body) if status.is_success() => match transcript(&body) {
                    Some(text) => return Ok(text),
                    None => failures.push(format!("{SIDECAR}: replied without a text field")),
                },
                Ok(body) => failures.push(format!("{SIDECAR}: http {status}: {}", refusal(&body))),
                Err(e) => failures.push(format!("{SIDECAR}: unreadable reply: {e}")),
            }
        }
        Err(e) => failures.push(format!("the speech sidecar at {SIDECAR} is not answering: {e}")),
    }
    // An OpenAI-shaped server on the same port (multipart `file`).
    for path in ["/v1/audio/transcriptions", "/transcribe"] {
        let url = format!("{SIDECAR}{path}");
        let form = reqwest::blocking::multipart::Form::new()
            .part("file", reqwest::blocking::multipart::Part::bytes(wav.clone())
                .file_name("audio.wav").mime_str("audio/wav")
                .map_err(|e| format!("bad multipart part: {e}"))?)
            .text("model", "whisper-1");
        match client.post(&url).multipart(form).send() {
            Ok(resp) => {
                let status = resp.status();
                match resp.text() {
                    Ok(body) if status.is_success() => {
                        if let Some(text) = transcript(&body) {
                            return Ok(text);
                        }
                    }
                    Ok(body) => failures.push(format!("{url}: http {status}: {}", refusal(&body))),
                    Err(e) => failures.push(format!("{url}: unreadable reply: {e}")),
                }
            }
            Err(e) => failures.push(format!("{url}: {e}")),
        }
    }
    Err(failures.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::refusal;

    #[test]
    fn a_refusal_shows_the_sentence_not_the_json() {
        assert_eq!(refusal(r#"{"ok":false,"error":"audio must be at most 16 MiB"}"#), "audio must be at most 16 MiB");
        assert_eq!(refusal(r#"{"detail":"file too large"}"#), "file too large");
    }

    #[test]
    fn a_plain_body_is_passed_through_as_written() {
        assert_eq!(refusal("  Gateway Error 502\n"), "Gateway Error 502");
        assert_eq!(refusal("{\"text\":\"not a refusal\"}"), r#"{"text":"not a refusal"}"#);
    }
}
