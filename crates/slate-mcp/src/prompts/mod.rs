use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDefinition {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: PromptContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PromptContent {
    #[serde(rename = "text")]
    Text { text: String },
}

pub fn list_prompts() -> Vec<PromptDefinition> {
    vec![
        PromptDefinition {
            name: "rough_cut".to_string(),
            description: "Assemble a complete rough cut from raw footage: import media, transcribe speech, remove silences/hesitations, cut out filler words, generate styled captions, and verify on timeline."
                .to_string(),
            arguments: vec![
                PromptArgument {
                    name: "media_path".to_string(),
                    description: "Path to the raw video file on disk".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "style".to_string(),
                    description: "Editing pacing: 'punchy' (tight pauses < 0.5s), 'conversational' (natural pauses), or 'documentary'".to_string(),
                    required: false,
                },
            ],
        },
        PromptDefinition {
            name: "make_shorts".to_string(),
            description: "Repurpose long-form video into vertical 9:16 short-form clips: find highest-energy highlights, center speaker, add animated karaoke captions."
                .to_string(),
            arguments: vec![
                PromptArgument {
                    name: "media_id".to_string(),
                    description: "Imported media asset ID".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target_duration_seconds".to_string(),
                    description: "Desired length of the short in seconds (e.g. 30 to 60)".to_string(),
                    required: false,
                },
            ],
        },
        PromptDefinition {
            name: "clean_audio".to_string(),
            description: "Clean and balance timeline audio: remove background room tone silences, normalize speech to -14 LUFS, and duck background music under vocals."
                .to_string(),
            arguments: vec![
                PromptArgument {
                    name: "voice_track_id".to_string(),
                    description: "Track ID containing speaker vocals".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "music_track_id".to_string(),
                    description: "Track ID containing background music".to_string(),
                    required: false,
                },
            ],
        },
    ]
}

pub fn get_prompt(name: &str, args: &serde_json::Value) -> Option<Vec<PromptMessage>> {
    match name {
        "rough_cut" => {
            let media_path = args.get("media_path").and_then(|v| v.as_str()).unwrap_or("");
            let style = args.get("style").and_then(|v| v.as_str()).unwrap_or("conversational");
            let prompt_text = format!(
                "You are an expert video editor assembling a rough cut.\n\
                 1. Call `media_import` with path: '{media_path}' and transcribe: true.\n\
                 2. Call `media_silences` to detect dead air. Style is '{style}'.\n\
                 3. Call `media_transcript` to review the dialogue. Note repetitive takes or filler phrases ('um', 'uh').\n\
                 4. Use `edit_by_transcript` with keep ranges to assemble the strongest takes onto the video track.\n\
                 5. Call `captions_generate` to generate aligned captions.\n\
                 6. Call `timeline_summary` and `render_frame` to visually verify your edit."
            );
            Some(vec![PromptMessage {
                role: "user".to_string(),
                content: PromptContent::Text { text: prompt_text },
            }])
        }
        "make_shorts" => {
            let media_id = args.get("media_id").and_then(|v| v.as_str()).unwrap_or("");
            let target_sec = args.get("target_duration_seconds").and_then(|v| v.as_f64()).unwrap_or(60.0);
            let prompt_text = format!(
                "You are a short-form video viral content editor.\n\
                 1. Inspect media asset '{media_id}'.\n\
                 2. Select high-energy highlight ranges totaling ~{target_sec} seconds.\n\
                 3. Reframe clip to 9:16 vertical crop.\n\
                 4. Generate punchy karaoke captions.\n\
                 5. Call `render_contact_sheet` to inspect visual pacing across the short."
            );
            Some(vec![PromptMessage {
                role: "user".to_string(),
                content: PromptContent::Text { text: prompt_text },
            }])
        }
        "clean_audio" => {
            let voice_track = args.get("voice_track_id").and_then(|v| v.as_str()).unwrap_or("");
            let music_track = args.get("music_track_id").and_then(|v| v.as_str()).unwrap_or("");
            let prompt_text = format!(
                "You are an audio mixing engineer.\n\
                 1. Clean dead air pauses on voice track '{voice_track}' using `audio_remove_silence`.\n\
                 2. Normalize dialogue vocals to -14.0 LUFS.\n\
                 3. Apply `audio_duck` on music track '{music_track}' under voice track '{voice_track}' with -16 dB attenuation."
            );
            Some(vec![PromptMessage {
                role: "user".to_string(),
                content: PromptContent::Text { text: prompt_text },
            }])
        }
        _ => None,
    }
}
