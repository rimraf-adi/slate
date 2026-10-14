use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: String,
}

pub fn list_resources() -> Vec<ResourceDefinition> {
    vec![
        ResourceDefinition {
            uri: "slate://project/timeline".to_string(),
            name: "Current Project Timeline".to_string(),
            description: "Full JSON serialization of the active project timeline, tracks, and clips".to_string(),
            mime_type: "application/json".to_string(),
        },
        ResourceDefinition {
            uri: "slate://history".to_string(),
            name: "Engine Operation History".to_string(),
            description: "Full undo/redo operation history log with agent/human provenance".to_string(),
            mime_type: "application/json".to_string(),
        },
        ResourceDefinition {
            uri: "slate://media/{id}/transcript".to_string(),
            name: "Media Speech Transcript".to_string(),
            description: "Word-level timestamped speech transcript for a media asset".to_string(),
            mime_type: "application/json".to_string(),
        },
        ResourceDefinition {
            uri: "slate://media/{id}/scenes".to_string(),
            name: "Media Scene Cuts".to_string(),
            description: "Visual scene transition and cut timestamps for a media asset".to_string(),
            mime_type: "application/json".to_string(),
        },
    ]
}
